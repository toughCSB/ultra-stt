use async_trait::async_trait;

use crate::error::AppError;

use super::{RemotePolishOptions, SttConfig, SttProvider, TranscriptEvent};

/// Configuration for a Whisper-compatible HTTP file-upload STT provider.
#[derive(Debug)]
pub struct WhisperCompatConfig {
    pub provider_name: String,
    pub endpoint: String,
    pub model: String,
    /// Extra form text fields (e.g. GLM-ASR needs "stream"="false").
    pub extra_fields: Vec<(String, String)>,
    /// Local OpenAI-compatible servers often do not require authentication.
    pub api_key_required: bool,
}

/// Max audio buffer: ~24 MB PCM ≈ 12.5 min at 16kHz 16-bit mono.
/// Keeps the resulting WAV under 25 MB (OpenAI/Groq limit).
const MAX_AUDIO_BYTES: usize = 24 * 1024 * 1024;

/// Generic provider for any OpenAI Whisper-compatible transcription API.
/// Works with: OpenAI, Groq, SiliconFlow, GLM-ASR.
pub struct WhisperCompatProvider {
    provider_config: WhisperCompatConfig,
    stt_config: Option<SttConfig>,
    audio_buffer: Vec<u8>,
    client: reqwest::Client,
    remote_polish: Option<RemotePolishOptions>,
    raw_transcript: Option<String>,
}

impl WhisperCompatProvider {
    pub fn new(provider_config: WhisperCompatConfig) -> Self {
        Self {
            provider_config,
            stt_config: None,
            audio_buffer: Vec::new(),
            client: reqwest::Client::new(),
            remote_polish: None,
            raw_transcript: None,
        }
    }

    pub fn with_client(provider_config: WhisperCompatConfig, client: reqwest::Client) -> Self {
        Self {
            provider_config,
            stt_config: None,
            audio_buffer: Vec::new(),
            client,
            remote_polish: None,
            raw_transcript: None,
        }
    }

    /// Peak sample and root mean square of a mono PCM16 buffer, on a 0..32767 scale.
    fn measure_level(pcm: &[u8]) -> (f64, f64) {
        let mut peak = 0f64;
        let mut sum = 0f64;
        let mut count = 0f64;
        for frame in pcm.chunks_exact(2) {
            let sample = i16::from_le_bytes([frame[0], frame[1]]) as f64;
            peak = peak.max(sample.abs());
            sum += sample * sample;
            count += 1.0;
        }
        let rms = if count > 0.0 {
            (sum / count).sqrt()
        } else {
            0.0
        };
        (peak, rms)
    }
    /// Build a WAV file from raw PCM 16-bit mono audio. Public so test helpers can reuse it.
    pub fn build_wav(pcm: &[u8], sample_rate: u32) -> Vec<u8> {
        let data_len = pcm.len() as u32;
        let channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * (channels as u32) * (bits_per_sample as u32) / 8;
        let block_align = channels * bits_per_sample / 8;
        let file_size = 36 + data_len;

        let mut wav = Vec::with_capacity(44 + pcm.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(pcm);
        wav
    }
}

#[async_trait]
impl SttProvider for WhisperCompatProvider {
    fn set_remote_polish(&mut self, options: &RemotePolishOptions) {
        self.remote_polish = Some(options.clone());
    }

    async fn connect(&mut self, config: &SttConfig) -> Result<(), AppError> {
        if self.provider_config.api_key_required && config.api_key.is_empty() {
            return Err(AppError::Auth(format!(
                "{} API key is empty",
                self.provider_config.provider_name
            )));
        }
        self.stt_config = Some(config.clone());
        self.raw_transcript = None;
        self.audio_buffer.clear();
        tracing::info!(
            "{} provider ready (buffering mode)",
            self.provider_config.provider_name
        );
        Ok(())
    }

    async fn send_audio(&mut self, chunk: &[u8]) -> Result<(), AppError> {
        if self.audio_buffer.len() + chunk.len() > MAX_AUDIO_BYTES {
            return Err(AppError::Config(format!(
                "{}: audio exceeds maximum length (~12 min)",
                self.provider_config.provider_name
            )));
        }
        self.audio_buffer.extend_from_slice(chunk);
        Ok(())
    }

    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>, AppError> {
        // File-based providers transcribe in disconnect(); keep this future
        // pending so the pipeline select loop does not busy-spin while recording.
        std::future::pending().await
    }

    async fn disconnect(&mut self) -> Result<Option<String>, AppError> {
        let config = match &self.stt_config {
            Some(c) => c.clone(),
            None => return Ok(None),
        };

        if self.audio_buffer.is_empty() {
            tracing::info!(
                "{}: no audio buffered, skipping",
                self.provider_config.provider_name
            );
            return Ok(None);
        }

        let audio_len_secs = self.audio_buffer.len() as f64 / (config.sample_rate as f64 * 2.0);
        let wav_data = Self::build_wav(&self.audio_buffer, config.sample_rate);
        // Report the level of what we captured. A dictation that returns no text is
        // almost always a silent capture, and the level turns that into evidence
        // instead of a guess.
        let (peak, rms) = Self::measure_level(&self.audio_buffer);
        self.audio_buffer.clear();
        tracing::info!(
            "{}: sending {:.1}s of audio, peak {:.0}, rms {:.0}",
            self.provider_config.provider_name,
            audio_len_secs,
            peak,
            rms
        );

        let request_budget =
            super::processing_budget::upload(super::processing_budget::pcm_seconds(
                wav_data.len().saturating_sub(44),
                config.sample_rate,
            ));
        let deadline = tokio::time::Instant::now() + request_budget;
        let mut attempt = 0u32;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(AppError::Timeout(request_budget));
            }
            let file_part = reqwest::multipart::Part::bytes(wav_data.clone())
                .file_name("audio.wav")
                .mime_str("audio/wav")
                .map_err(|e| AppError::Config(e.to_string()))?;

            let mut form = reqwest::multipart::Form::new()
                .text("model", self.provider_config.model.to_string())
                .part("file", file_part);

            // Language hint (OpenAI/Groq support `language` field, others use `prompt`)
            if let Some(ref lang) = config.language {
                if lang != "multi" {
                    form = form.text("language", lang.clone());
                }
            }

            // Provider-specific extra fields
            for (key, value) in &self.provider_config.extra_fields {
                form = form.text(key.clone(), value.clone());
            }

            let mut request = self
                .client
                .post(&self.provider_config.endpoint)
                .multipart(form)
                .timeout(remaining);

            if !config.api_key.trim().is_empty() {
                request = request.header("Authorization", format!("Bearer {}", config.api_key));
            }

            if let Some(options) = &self.remote_polish {
                request = request
                    .header("x-ultra-stt-polish-style", &options.style)
                    .header("x-ultra-stt-polish-enabled", options.enabled.to_string());
            }

            let resp_result = request.send().await;

            match resp_result {
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();

                    if status.is_success() {
                        let v: serde_json::Value = serde_json::from_str(&body)
                            .map_err(|e| AppError::Config(e.to_string()))?;
                        if v.get("error").is_some_and(|error| !error.is_null()) {
                            return Err(AppError::Config(
                                "Speech recognition server returned an error instead of a transcript"
                                    .to_string(),
                            ));
                        }
                        let text = v
                            .get("text")
                            .and_then(|text| text.as_str())
                            .ok_or_else(|| {
                                AppError::Config(
                                    "Speech recognition response has no text field".to_string(),
                                )
                            })?
                            .trim()
                            .to_string();

                        self.raw_transcript = self.remote_polish.as_ref().and_then(|_| {
                            v.get("raw")
                                .and_then(|value| value.as_str())
                                .filter(|raw| !raw.trim().is_empty())
                                .map(str::to_string)
                        });

                        tracing::info!(
                            "{} transcription: {} chars",
                            self.provider_config.provider_name,
                            text.len()
                        );

                        return Ok(if text.is_empty() { None } else { Some(text) });
                    } else if status.as_u16() >= 500 && attempt < 2 {
                        let truncate_at = body
                            .char_indices()
                            .take_while(|&(i, _)| i < 200)
                            .last()
                            .map(|(i, c)| i + c.len_utf8())
                            .unwrap_or(body.len());
                        tracing::warn!(
                            "{} server error {} (attempt {}/3): {}",
                            self.provider_config.provider_name,
                            status,
                            attempt + 1,
                            &body[..truncate_at]
                        );
                        attempt += 1;
                        tokio::time::sleep(std::time::Duration::from_millis(
                            1000 * 2u64.pow(attempt - 1),
                        ))
                        .await;
                        continue;
                    } else {
                        // Truncate at a valid UTF-8 char boundary to avoid panic on multi-byte chars
                        let truncate_at = body
                            .char_indices()
                            .take_while(|&(i, _)| i < 200)
                            .last()
                            .map(|(i, c)| i + c.len_utf8())
                            .unwrap_or(body.len());
                        let sanitized = &body[..truncate_at];
                        tracing::error!(
                            "{} HTTP {}: {}",
                            self.provider_config.provider_name,
                            status,
                            sanitized
                        );
                        return Err(AppError::Api {
                            status: status.as_u16(),
                            body: sanitized.to_string(),
                        });
                    }
                }
                Err(e) if e.is_timeout() => return Err(AppError::Timeout(request_budget)),
                Err(e) => return Err(e.into()),
            }
        }
    }

    fn name(&self) -> &str {
        &self.provider_config.provider_name
    }

    fn raw_transcript(&self) -> Option<String> {
        self.raw_transcript.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn transcribe_http_reply(body: &'static str) -> Result<Option<String>, AppError> {
        transcribe_http_reply_with_options(body, None).await
    }

    async fn transcribe_http_reply_with_options(
        body: &'static str,
        options: Option<RemotePolishOptions>,
    ) -> Result<Option<String>, AppError> {
        transcribe_audio_reply(body, options, 2).await
    }

    async fn transcribe_audio_reply(
        body: &'static str,
        options: Option<RemotePolishOptions>,
        audio_bytes: usize,
    ) -> Result<Option<String>, AppError> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/transcriptions", listener.local_addr().unwrap());
        let expected_options = options.clone();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 4096];
            loop {
                let count = stream.read(&mut chunk).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&chunk[..count]);
                if let Some(head_end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&request[..head_end]);
                    let length = head
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if request.len() >= head_end + 4 + length {
                        let wav_start =
                            request.windows(4).position(|part| part == b"RIFF").unwrap();
                        let pcm_length = u32::from_le_bytes(
                            request[wav_start + 40..wav_start + 44].try_into().unwrap(),
                        ) as usize;
                        assert_eq!(pcm_length, audio_bytes);
                        assert_eq!(
                            request[wav_start + 44..wav_start + 44 + pcm_length],
                            vec![0; audio_bytes]
                        );
                        match &expected_options {
                            Some(options) => {
                                assert!(head.contains(&format!(
                                    "x-ultra-stt-polish-style: {}",
                                    options.style
                                )));
                                assert!(head.contains(&format!(
                                    "x-ultra-stt-polish-enabled: {}",
                                    options.enabled
                                )));
                            }
                            None => assert!(!head.contains("x-ultra-stt-polish-")),
                        }
                        break;
                    }
                }
            }
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        });
        let mut provider = WhisperCompatProvider::with_client(
            WhisperCompatConfig {
                provider_name: "test-whisper".to_string(),
                endpoint,
                model: "test-model".to_string(),
                extra_fields: vec![],
                api_key_required: false,
            },
            reqwest::Client::builder().no_proxy().build().unwrap(),
        );
        provider
            .connect(&SttConfig {
                api_key: String::new(),
                language: None,
                smart_format: true,
                sample_rate: 16000,
                resource_id: None,
                operation_id: None,
                managed_audio: None,
                provider_region: None,
            })
            .await
            .unwrap();
        provider.send_audio(&vec![0; audio_bytes]).await.unwrap();
        if let Some(options) = &options {
            provider.set_remote_polish(&RemotePolishOptions::default());
            provider.set_remote_polish(options);
        }
        let result = tokio::time::timeout(std::time::Duration::from_secs(3), provider.disconnect())
            .await
            .unwrap();
        let response: serde_json::Value = serde_json::from_str(body).unwrap();
        let expected_raw = options
            .as_ref()
            .and_then(|_| response.get("raw").and_then(|value| value.as_str()))
            .map(str::to_string);
        assert_eq!(provider.raw_transcript(), expected_raw);
        server.await.unwrap();
        result
    }

    #[tokio::test]
    async fn ten_minute_recording_upload_preserves_every_pcm_byte() {
        assert_eq!(
            transcribe_audio_reply(
                r#"{"text":"complete ten minute transcript"}"#,
                None,
                600 * 32_000
            )
            .await
            .unwrap(),
            Some("complete ten minute transcript".to_string())
        );
    }

    #[tokio::test]
    async fn remote_headers_use_the_final_mode_and_do_not_leak_to_other_requests() {
        for style in crate::storage::POLISH_STYLES {
            let options = RemotePolishOptions {
                style: style.to_string(),
                enabled: false,
            };
            assert_eq!(
                transcribe_http_reply_with_options(r#"{"text":"원문"}"#, Some(options))
                    .await
                    .unwrap(),
                Some("원문".to_string())
            );
        }
        assert_eq!(
            transcribe_http_reply(r#"{"text":"일반"}"#).await.unwrap(),
            Some("일반".to_string())
        );
    }

    #[tokio::test]
    async fn remote_original_is_kept_separate_from_the_polished_result() {
        assert_eq!(
            transcribe_http_reply_with_options(
                r#"{"raw":"정확한 전사 원문","text":"다듬은 결과"}"#,
                Some(RemotePolishOptions::default())
            )
            .await
            .unwrap(),
            Some("다듬은 결과".to_string())
        );
        assert_eq!(
            transcribe_http_reply(r#"{"raw":"not a remote response","text":"일반 결과"}"#)
                .await
                .unwrap(),
            Some("일반 결과".to_string())
        );
    }

    #[tokio::test]
    async fn http_success_with_error_payload_is_not_silently_treated_as_no_speech() {
        assert!(
            transcribe_http_reply(r#"{"error":"voice input could not be processed"}"#)
                .await
                .is_err()
        );
        assert!(
            transcribe_http_reply(r#"{"error":{"message":"failed"},"text":""}"#)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn http_success_without_transcript_field_is_an_error() {
        assert!(transcribe_http_reply(r#"{"status":"failed"}"#)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn http_success_preserves_transcript_and_legitimate_empty_speech() {
        assert_eq!(
            transcribe_http_reply(r#"{"text":" 테스트 "}"#)
                .await
                .unwrap(),
            Some("테스트".to_string())
        );
        assert_eq!(transcribe_http_reply(r#"{"text":""}"#).await.unwrap(), None);
    }

    #[tokio::test]
    async fn connect_allows_empty_api_key_when_not_required() {
        let mut provider = WhisperCompatProvider::new(WhisperCompatConfig {
            provider_name: "custom-whisper".to_string(),
            endpoint: "http://localhost:8000/v1/audio/transcriptions".to_string(),
            model: "test-model".to_string(),
            extra_fields: vec![],
            api_key_required: false,
        });

        let result = provider
            .connect(&SttConfig {
                api_key: String::new(),
                language: None,
                smart_format: true,
                sample_rate: 16000,
                resource_id: None,
                operation_id: None,
                managed_audio: None,
                provider_region: None,
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn recv_transcript_waits_for_file_based_provider() {
        let mut provider = WhisperCompatProvider::new(WhisperCompatConfig {
            provider_name: "test-whisper".to_string(),
            endpoint: "https://example.test/transcriptions".to_string(),
            model: "test-model".to_string(),
            extra_fields: vec![],
            api_key_required: true,
        });

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(20),
            provider.recv_transcript(),
        )
        .await;

        assert!(result.is_err());
    }
}
