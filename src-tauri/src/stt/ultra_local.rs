//! Owned project-local CUDA speech service. Taking the owner out of the warm slot
//! ensures dropping a cancelled inference closes the Job instead of retaining GPU work.
use super::{whisper_compat::WhisperCompatProvider, SttConfig, SttProvider, TranscriptEvent};
use crate::{
    error::AppError,
    local_process::{Limits, LocalProcess},
};
use async_trait::async_trait;
use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

pub const BASE: &str = "http://127.0.0.1:17845/v1";
pub const MODEL: &str = "Systran/faster-whisper-large-v3";
const ENDPOINT: &str = "http://127.0.0.1:17845/v1/audio/transcriptions";
const MAX_PCM: usize = 24 * 1024 * 1024;
struct Service {
    owner: LocalProcess,
    secret: String,
}
static SERVICE: OnceLock<Mutex<Option<Service>>> = OnceLock::new();
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn service() -> &'static Mutex<Option<Service>> {
    SERVICE.get_or_init(|| Mutex::new(None))
}
fn error(detail: impl std::fmt::Display) -> AppError {
    AppError::Config(format!("Local speech service: {detail}"))
}
fn client() -> Result<reqwest::Client, AppError> {
    if let Some(client) = CLIENT.get() {
        return Ok(client.clone());
    }
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(error)?;
    let _ = CLIENT.set(client.clone());
    Ok(client)
}
fn root() -> Result<std::path::PathBuf, AppError> {
    let path = std::env::var_os("ULTRA_STT_ROOT")
        .ok_or_else(|| error("ULTRA_STT_ROOT is unset; launch via start-ultra-stt.ps1"))?;
    let path = std::path::PathBuf::from(path);
    if !path.is_absolute() {
        return Err(error("ULTRA_STT_ROOT must be absolute"));
    }
    Ok(path)
}
async fn start() -> Result<Service, AppError> {
    let started = Instant::now();
    let root = root()?;
    let interpreter = root.join(".venv/Scripts/python.exe");
    let app = root.join("opentypeless");
    let secret = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let limits = Limits {
        stdin: 256,
        stdout: 512,
        stderr: 16 * 1024,
        deadline: Duration::from_secs(120),
    };
    let mut owner = LocalProcess::spawn(interpreter, limits, |command| {
        command
            .args(["-m", "local_stt.server"])
            .current_dir(&app)
            .env("ULTRA_STT_ROOT", &root)
            .env("HF_HUB_OFFLINE", "1");
    })
    .map_err(error)?;
    let bootstrap = serde_json::json!({"protocol":1,"session_secret":secret}).to_string() + "\n";
    let deadline = Instant::now() + limits.deadline;
    owner
        .send_stdin(bootstrap.as_bytes(), limits.stdin, limits.deadline)
        .await
        .map_err(error)?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    let line = owner
        .ready_line(limits.stdout, remaining)
        .await
        .map_err(error)?;
    let ready: serde_json::Value = serde_json::from_slice(&line).map_err(error)?;
    if ready["event"] != "ready"
        || ready["protocol"] != 1
        || ready["endpoint"] != BASE
        || ready["pid"].as_u64().is_none_or(|pid| pid == 0)
    {
        return Err(error("invalid helper readiness event"));
    }
    tracing::info!(
        startup_ms = started.elapsed().as_millis() as u64,
        "Local speech service ready"
    );
    Ok(Service { owner, secret })
}

pub async fn warm() -> Result<(), AppError> {
    let mut slot = service().lock().await;
    if slot.is_none() {
        *slot = Some(start().await?);
    }
    Ok(())
}

pub async fn transcribe(pcm: &[u8], language: Option<&str>) -> Result<Option<String>, AppError> {
    request(pcm, language, None).await
}

/// Fast greedy pass for live typing; `prompt` carries earlier typed text and
/// `hotwords` the user's dictionary terms.
pub async fn transcribe_live(
    pcm: &[u8],
    language: Option<&str>,
    prompt: &str,
    hotwords: &str,
) -> Result<Option<String>, AppError> {
    request(pcm, language, Some((prompt, hotwords))).await
}

async fn request(
    pcm: &[u8],
    language: Option<&str>,
    live_hints: Option<(&str, &str)>,
) -> Result<Option<String>, AppError> {
    if pcm.len() > MAX_PCM || !pcm.len().is_multiple_of(2) {
        return Err(error("invalid PCM length"));
    }
    if pcm.is_empty() {
        return Ok(None);
    }
    let mut slot = service().lock().await;
    let owned = match slot.take() {
        Some(owned) => owned,
        None => start().await?,
    };
    // If this future is cancelled, `owned` is dropped and its Job kills all descendants.
    let wav = WhisperCompatProvider::build_wav(pcm, 16000);
    let file = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(error)?;
    let mut form = reqwest::multipart::Form::new()
        .text("model", MODEL)
        .part("file", file);
    if let Some(lang) = language.filter(|lang| *lang != "multi") {
        form = form.text("language", lang.to_string());
    }
    if let Some((prompt, hotwords)) = live_hints {
        form = form.text("mode", "live");
        if !prompt.is_empty() {
            form = form.text("prompt", prompt.to_string());
        }
        if !hotwords.is_empty() {
            form = form.text("hotwords", hotwords.to_string());
        }
    }
    let request_start = Instant::now();
    let response = client()?
        .post(ENDPOINT)
        .timeout(super::processing_budget::recognition(
            super::processing_budget::pcm_seconds(pcm.len(), 16000),
        ))
        .header("Host", "127.0.0.1:17845")
        .header("X-Ultra-STT-Session", &owned.secret)
        .multipart(form)
        .send()
        .await
        .map_err(error)?;
    if !response.status().is_success() {
        return Err(error(format!("HTTP {}", response.status())));
    }
    let payload: serde_json::Value = response.json().await.map_err(error)?;
    let text = payload["text"]
        .as_str()
        .ok_or_else(|| error("missing transcription"))?
        .trim()
        .to_owned();
    tracing::info!(
        request_ms = request_start.elapsed().as_millis() as u64,
        audio_ms = pcm.len() / 32,
        "Local speech transcription completed"
    );
    *slot = Some(owned); // successful inference retains the ready CUDA model
    Ok((!text.is_empty()).then_some(text))
}

#[derive(Default)]
pub struct UltraLocalProvider {
    buffer: Vec<u8>,
    config: Option<SttConfig>,
}
impl UltraLocalProvider {
    pub fn new() -> Self {
        Self::default()
    }
}
#[async_trait]
impl SttProvider for UltraLocalProvider {
    async fn connect(&mut self, config: &SttConfig) -> Result<(), AppError> {
        if config.sample_rate != 16000 {
            return Err(error("16 kHz PCM is required"));
        }
        let mut slot = service().lock().await;
        if slot.is_none() {
            *slot = Some(start().await?);
        }
        self.config = Some(config.clone());
        self.buffer.clear();
        Ok(())
    }
    async fn send_audio(&mut self, chunk: &[u8]) -> Result<(), AppError> {
        if self.buffer.len().saturating_add(chunk.len()) > MAX_PCM {
            return Err(error("PCM limit exceeded"));
        }
        self.buffer.extend_from_slice(chunk);
        Ok(())
    }
    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>, AppError> {
        std::future::pending().await
    }
    async fn disconnect(&mut self) -> Result<Option<String>, AppError> {
        let config = self.config.take();
        let pcm = std::mem::take(&mut self.buffer);
        transcribe(
            &pcm,
            config
                .as_ref()
                .and_then(|config| config.language.as_deref()),
        )
        .await
    }
    fn name(&self) -> &str {
        "custom-whisper"
    }
}

/// Live typing timings, in milliseconds of 16 kHz audio.
const LIVE_PASS_INTERVAL: Duration = Duration::from_millis(450);
const LIVE_POLL: Duration = Duration::from_millis(40);
const LIVE_MIN_WINDOW_MS: usize = 800;
const LIVE_MIN_SPEECH_MS: usize = 250;
/// Silence that closes an utterance; its words are then typed without waiting.
const LIVE_PAUSE_MS: usize = 700;
const LIVE_MAX_UTTERANCE_MS: usize = 20_000;
/// Speech below this many milliseconds is too short to trust a lone stock phrase.
const LIVE_HALLUCINATION_GUARD_MS: usize = 900;
const LIVE_PROMPT_CHARS: usize = 120;
/// The local service accepts at most 400 characters per hint field.
const LIVE_HOTWORD_CHARS: usize = 400;
const LIVE_MAX_ERRORS: u32 = 3;
const LIVE_REMOTE_TIMEOUT: Duration = Duration::from_secs(10);
const FRAME_SAMPLES: usize = 320; // 20 ms
const SPEECH_RMS: f32 = 0.012;

fn pcm_ms(bytes: usize) -> usize {
    bytes / 32
}

/// Dictionary terms as a Whisper hotword list, whole terms only, within the service limit.
fn live_hotwords(words: &[String]) -> String {
    let mut hotwords = String::new();
    for word in words
        .iter()
        .map(|word| word.trim())
        .filter(|word| !word.is_empty())
    {
        let separator = if hotwords.is_empty() { "" } else { ", " };
        if hotwords.chars().count() + separator.chars().count() + word.chars().count()
            > LIVE_HOTWORD_CHARS
        {
            break;
        }
        hotwords.push_str(separator);
        hotwords.push_str(word);
    }
    hotwords
}

/// User correction rules (pattern -> replacement), applied before text is typed.
fn apply_corrections(text: &str, rules: &[(String, String)]) -> String {
    rules
        .iter()
        .filter(|(pattern, _)| !pattern.is_empty())
        .fold(text.to_string(), |text, (pattern, replacement)| {
            text.replace(pattern.as_str(), replacement)
        })
}

/// Where live passes run: this PC's model, or the owner's recognition PC over the tailnet.
#[derive(Clone, Default)]
pub enum LiveBackend {
    #[default]
    Local,
    Remote {
        endpoint: String,
        token: String,
        client: reqwest::Client,
    },
}

/// The remote live endpoint sits next to the remote transcription endpoint.
pub fn live_endpoint_for(transcription_endpoint: &str) -> String {
    let base = transcription_endpoint.trim_end_matches('/');
    let base = base.strip_suffix("/audio/transcriptions").unwrap_or(base);
    format!("{base}/audio/live")
}

async fn remote_live(
    endpoint: &str,
    token: &str,
    client: &reqwest::Client,
    pcm: &[u8],
    prompt: &str,
    hotwords: &str,
) -> Result<Option<String>, AppError> {
    let wav = WhisperCompatProvider::build_wav(pcm, 16000);
    let file = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(error)?;
    let mut form = reqwest::multipart::Form::new().part("file", file);
    if !prompt.is_empty() {
        form = form.text("prompt", prompt.to_string());
    }
    if !hotwords.is_empty() {
        form = form.text("hotwords", hotwords.to_string());
    }
    let response = client
        .post(endpoint)
        .bearer_auth(token)
        .timeout(LIVE_REMOTE_TIMEOUT)
        .multipart(form)
        .send()
        .await
        .map_err(error)?;
    if !response.status().is_success() {
        return Err(error(format!("remote live HTTP {}", response.status())));
    }
    let payload: serde_json::Value = response.json().await.map_err(error)?;
    let text = payload["text"]
        .as_str()
        .ok_or_else(|| error("missing remote live transcription"))?
        .trim()
        .to_owned();
    Ok((!text.is_empty()).then_some(text))
}

/// Audio of the utterance being recognised plus a simple energy-based speech detector.
#[derive(Default)]
struct LiveAudio {
    pcm: Vec<u8>,
    frame: Vec<i16>,
    speech_ms: usize,
    trailing_silence_ms: usize,
    /// Silence since the last speech frame of the whole session (not reset per utterance).
    idle_ms: usize,
    stopping: bool,
}

impl LiveAudio {
    fn push(&mut self, chunk: &[u8]) {
        self.pcm.extend_from_slice(chunk);
        for sample in chunk.chunks_exact(2) {
            self.frame.push(i16::from_le_bytes([sample[0], sample[1]]));
            if self.frame.len() == FRAME_SAMPLES {
                let energy: f32 = self
                    .frame
                    .iter()
                    .map(|&s| (f32::from(s) / 32768.0).powi(2))
                    .sum();
                if (energy / FRAME_SAMPLES as f32).sqrt() >= SPEECH_RMS {
                    self.speech_ms += 20;
                    self.trailing_silence_ms = 0;
                    self.idle_ms = 0;
                } else {
                    self.trailing_silence_ms += 20;
                    self.idle_ms += 20;
                }
                self.frame.clear();
            }
        }
        // Leading silence carries no words; keep only a short lead-in before speech.
        if self.speech_ms == 0 && self.pcm.len() > 16_000 {
            let excess = self.pcm.len() - 9_600;
            self.pcm.drain(..excess);
        }
    }

    /// Remove the first `bytes` (already recognised) and restart speech detection.
    fn consume(&mut self, bytes: usize) {
        self.pcm.drain(..bytes.min(self.pcm.len()));
        self.speech_ms = 0;
        self.trailing_silence_ms = 0;
    }
}

enum LiveStep {
    Wait,
    Pass(Vec<u8>, usize),
    Finish(Vec<u8>, usize),
    Stop,
}

fn next_live_step(audio: &LiveAudio, since_pass: Duration, recognised_bytes: usize) -> LiveStep {
    let utterance_ms = pcm_ms(audio.pcm.len());
    let has_speech = audio.speech_ms >= LIVE_MIN_SPEECH_MS;
    if audio.stopping {
        return if has_speech {
            LiveStep::Finish(audio.pcm.clone(), audio.speech_ms)
        } else {
            LiveStep::Stop
        };
    }
    if has_speech
        && (audio.trailing_silence_ms >= LIVE_PAUSE_MS || utterance_ms >= LIVE_MAX_UTTERANCE_MS)
    {
        return LiveStep::Finish(audio.pcm.clone(), audio.speech_ms);
    }
    if has_speech
        && utterance_ms >= LIVE_MIN_WINDOW_MS
        && since_pass >= LIVE_PASS_INTERVAL
        && audio.pcm.len() > recognised_bytes
    {
        return LiveStep::Pass(audio.pcm.clone(), audio.speech_ms);
    }
    LiveStep::Wait
}

struct LiveWorker {
    audio: std::sync::Arc<std::sync::Mutex<LiveAudio>>,
    events: tokio::sync::mpsc::UnboundedSender<TranscriptEvent>,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    language: Option<String>,
    backend: LiveBackend,
    /// Silence that ends the session; 0 keeps it open.
    idle_stop_ms: usize,
    hotwords: String,
    corrections: Vec<(String, String)>,
    agreement: super::live_agreement::LiveAgreement,
    typed: String,
    /// Length of `typed` when the current utterance began. Only earlier text is used as
    /// the prompt: Whisper skips audio it believes the prompt already contains.
    utterance_start: usize,
    errors: u32,
}

impl LiveWorker {
    fn prompt(&self) -> String {
        let before = &self.typed[..self.utterance_start];
        let skip = before.chars().count().saturating_sub(LIVE_PROMPT_CHARS);
        before.chars().skip(skip).collect()
    }

    fn commit(&mut self, words: Vec<String>) {
        if words.is_empty() {
            return;
        }
        let separator = if self.typed.is_empty() { "" } else { " " };
        let text = format!(
            "{separator}{}",
            apply_corrections(&words.join(" "), &self.corrections)
        );
        self.typed.push_str(&text);
        let _ = self.events.send(TranscriptEvent::Committed { text });
    }

    fn preview(&self) {
        let pending = self.agreement.pending().join(" ");
        let text = [self.typed.as_str(), pending.as_str()]
            .iter()
            .filter(|part| !part.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        let _ = self.events.send(TranscriptEvent::Partial { text });
    }

    /// Recognise `pcm`; a failure is retried on the next pass until it repeats.
    async fn recognise(
        &mut self,
        pcm: &[u8],
        speech_ms: usize,
    ) -> Result<Option<String>, AppError> {
        let prompt = self.prompt();
        let result = match &self.backend {
            LiveBackend::Local => {
                transcribe_live(pcm, self.language.as_deref(), &prompt, &self.hotwords).await
            }
            LiveBackend::Remote {
                endpoint,
                token,
                client,
            } => remote_live(endpoint, token, client, pcm, &prompt, &self.hotwords).await,
        };
        match result {
            Ok(text) => {
                self.errors = 0;
                Ok(text.filter(|text| {
                    speech_ms >= LIVE_HALLUCINATION_GUARD_MS
                        || !super::live_agreement::is_hallucination(text)
                }))
            }
            Err(error) => {
                self.errors += 1;
                tracing::warn!(
                    attempt = self.errors,
                    "Live recognition pass failed: {error}"
                );
                if self.errors >= LIVE_MAX_ERRORS {
                    return Err(error);
                }
                Ok(None)
            }
        }
    }

    async fn run(mut self) -> String {
        let mut last_pass = Instant::now() - LIVE_PASS_INTERVAL;
        let mut recognised_bytes = 0usize;
        let mut idle_reported = false;
        loop {
            if self.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            let (step, idle) = {
                let audio = self.audio.lock().unwrap_or_else(|e| e.into_inner());
                (
                    next_live_step(&audio, last_pass.elapsed(), recognised_bytes),
                    !audio.stopping && self.idle_stop_ms > 0 && audio.idle_ms >= self.idle_stop_ms,
                )
            };
            if idle && !idle_reported {
                idle_reported = true;
                let _ = self.events.send(TranscriptEvent::SpeechEnded);
            }
            match step {
                LiveStep::Wait => tokio::time::sleep(LIVE_POLL).await,
                LiveStep::Stop => break,
                LiveStep::Pass(pcm, speech_ms) => {
                    last_pass = Instant::now();
                    recognised_bytes = pcm.len();
                    match self.recognise(&pcm, speech_ms).await {
                        Ok(Some(text)) => {
                            let words = self.agreement.update(&text);
                            self.commit(words);
                            self.preview();
                        }
                        Ok(None) => {}
                        Err(error) => {
                            let _ = self.events.send(TranscriptEvent::Error {
                                message: error.to_string(),
                            });
                            break;
                        }
                    }
                }
                LiveStep::Finish(pcm, speech_ms) => {
                    let stopping = self
                        .audio
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .stopping;
                    let text = match self.recognise(&pcm, speech_ms).await {
                        Ok(text) => text.unwrap_or_default(),
                        Err(error) => {
                            let _ = self.events.send(TranscriptEvent::Error {
                                message: error.to_string(),
                            });
                            break;
                        }
                    };
                    let words = self.agreement.finish(&text);
                    self.commit(words);
                    self.preview();
                    self.audio
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .consume(pcm.len());
                    recognised_bytes = 0;
                    self.utterance_start = self.typed.len();
                    if stopping {
                        break;
                    }
                }
            }
        }
        self.typed
    }
}

/// Live typing provider: words are emitted as `Committed` while the user speaks.
/// It reuses the warm local model with short greedy passes over the current utterance.
#[derive(Default)]
pub struct UltraLocalLiveProvider {
    audio: std::sync::Arc<std::sync::Mutex<LiveAudio>>,
    events: Option<tokio::sync::mpsc::UnboundedReceiver<TranscriptEvent>>,
    worker: Option<tokio::task::JoinHandle<String>>,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    hotwords: String,
    corrections: Vec<(String, String)>,
    backend: LiveBackend,
    idle_stop_ms: usize,
}

impl UltraLocalLiveProvider {
    /// `dictionary` terms guide recognition; `corrections` fix text before it is typed.
    pub fn new(
        dictionary: &[String],
        corrections: Vec<(String, String)>,
        backend: LiveBackend,
        idle_stop_ms: usize,
    ) -> Self {
        Self {
            hotwords: live_hotwords(dictionary),
            corrections,
            backend,
            idle_stop_ms,
            audio: Default::default(),
            events: None,
            worker: None,
            cancelled: Default::default(),
        }
    }
}

impl Drop for UltraLocalLiveProvider {
    fn drop(&mut self) {
        // An in-flight pass completes on its own; aborting it would kill the warm model.
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[async_trait]
impl SttProvider for UltraLocalLiveProvider {
    async fn connect(&mut self, config: &SttConfig) -> Result<(), AppError> {
        if config.sample_rate != 16000 {
            return Err(error("16 kHz PCM is required"));
        }
        if matches!(self.backend, LiveBackend::Local) {
            warm().await?;
        }
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        self.events = Some(receiver);
        let worker = LiveWorker {
            audio: self.audio.clone(),
            events: sender,
            cancelled: self.cancelled.clone(),
            language: config.language.clone(),
            backend: self.backend.clone(),
            idle_stop_ms: self.idle_stop_ms,
            hotwords: self.hotwords.clone(),
            corrections: self.corrections.clone(),
            agreement: Default::default(),
            typed: String::new(),
            utterance_start: 0,
            errors: 0,
        };
        self.worker = Some(tokio::spawn(worker.run()));
        Ok(())
    }
    async fn send_audio(&mut self, chunk: &[u8]) -> Result<(), AppError> {
        let mut audio = self.audio.lock().unwrap_or_else(|e| e.into_inner());
        if audio.pcm.len().saturating_add(chunk.len()) > MAX_PCM {
            return Err(error("PCM limit exceeded"));
        }
        audio.push(chunk);
        Ok(())
    }
    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>, AppError> {
        match self.events.as_mut() {
            Some(events) => match events.recv().await {
                Some(event) => Ok(Some(event)),
                None => std::future::pending().await,
            },
            None => std::future::pending().await,
        }
    }
    async fn disconnect(&mut self) -> Result<Option<String>, AppError> {
        self.audio
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .stopping = true;
        let Some(worker) = self.worker.take() else {
            return Ok(None);
        };
        let typed = worker.await.map_err(error)?;
        Ok((!typed.is_empty()).then_some(typed))
    }
    fn name(&self) -> &str {
        "custom-whisper-live"
    }
}

pub async fn shutdown() {
    let mut slot = service().lock().await;
    if let Some(mut owned) = slot.take() {
        let _ = owned.owner.kill(Duration::from_secs(5)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn repeated_local_requests_reuse_one_tcp_connection() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut connection, _) = listener.accept().await.unwrap();
            for _request in 0..2 {
                let mut head = Vec::new();
                while !head.ends_with(b"\r\n\r\n") {
                    head.push(connection.read_u8().await.unwrap());
                    assert!(head.len() < 8192);
                }
                connection
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
                    .await
                    .unwrap();
            }
        });
        let requests = async {
            for _request in 0..2 {
                let response = client().unwrap().get(&endpoint).send().await.unwrap();
                assert_eq!(response.bytes().await.unwrap().as_ref(), b"OK");
            }
            server.await.unwrap();
        };
        tokio::time::timeout(Duration::from_secs(3), requests)
            .await
            .unwrap();
    }

    #[test]
    fn local_endpoint_is_exact() {
        assert_eq!(ENDPOINT, format!("{BASE}/audio/transcriptions"));
        assert_eq!(MODEL, "Systran/faster-whisper-large-v3");
    }

    fn pcm(ms: usize, amplitude: i16) -> Vec<u8> {
        (0..ms * 16)
            .flat_map(|i| {
                let sample = if i % 2 == 0 { amplitude } else { -amplitude };
                sample.to_le_bytes()
            })
            .collect()
    }

    fn step_name(step: LiveStep) -> &'static str {
        match step {
            LiveStep::Wait => "wait",
            LiveStep::Pass(..) => "pass",
            LiveStep::Finish(..) => "finish",
            LiveStep::Stop => "stop",
        }
    }

    #[test]
    fn remote_live_endpoint_sits_next_to_transcriptions() {
        assert_eq!(
            live_endpoint_for("https://pc.example.ts.net:10443/v1/audio/transcriptions"),
            "https://pc.example.ts.net:10443/v1/audio/live"
        );
        assert_eq!(
            live_endpoint_for("https://pc.example.ts.net:10443/v1/"),
            "https://pc.example.ts.net:10443/v1/audio/live"
        );
    }

    #[test]
    fn idle_time_spans_utterances_and_resets_on_speech() {
        let mut audio = LiveAudio::default();
        audio.push(&pcm(500, 8_000));
        audio.push(&pcm(1_000, 0));
        audio.consume(audio.pcm.len());
        audio.push(&pcm(1_000, 0));
        assert_eq!(audio.idle_ms, 2_000);
        audio.push(&pcm(100, 8_000));
        assert_eq!(audio.idle_ms, 0);
    }

    #[test]
    fn dictionary_hotwords_keep_whole_terms_within_the_service_limit() {
        let words = vec![" 슬관절 ".to_string(), String::new(), "비급여".to_string()];
        assert_eq!(live_hotwords(&words), "슬관절, 비급여");
        let many: Vec<String> = (0..200).map(|i| format!("용어{i:03}")).collect();
        let hotwords = live_hotwords(&many);
        assert!(hotwords.chars().count() <= LIVE_HOTWORD_CHARS);
        assert!(hotwords
            .split(", ")
            .all(|term| term.starts_with("용어") && term.len() == "용어000".len()));
    }

    #[test]
    fn correction_rules_rewrite_text_in_order_and_skip_empty_patterns() {
        let rules = vec![
            ("다리미".to_string(), "다리 및".to_string()),
            (String::new(), "x".to_string()),
            ("오픈 타입".to_string(), "OpenType".to_string()),
        ];
        assert_eq!(
            apply_corrections("다리미 수직 간격 오픈 타입", &rules),
            "다리 및 수직 간격 OpenType"
        );
    }

    #[test]
    fn leading_silence_is_trimmed_and_never_recognised() {
        let mut audio = LiveAudio::default();
        audio.push(&pcm(3_000, 0));
        assert_eq!(pcm_ms(audio.pcm.len()), 300);
        assert_eq!(audio.speech_ms, 0);
        assert_eq!(
            step_name(next_live_step(&audio, LIVE_PASS_INTERVAL, 0)),
            "wait"
        );
    }

    #[test]
    fn speech_triggers_paced_passes_then_a_pause_finishes_the_utterance() {
        let mut audio = LiveAudio::default();
        audio.push(&pcm(1_000, 8_000));
        assert_eq!(audio.speech_ms, 1_000);
        assert_eq!(
            step_name(next_live_step(&audio, LIVE_PASS_INTERVAL, 0)),
            "pass"
        );
        // Too soon after the previous pass, or nothing new since it.
        assert_eq!(
            step_name(next_live_step(&audio, Duration::from_millis(100), 0)),
            "wait"
        );
        assert_eq!(
            step_name(next_live_step(&audio, LIVE_PASS_INTERVAL, audio.pcm.len())),
            "wait"
        );
        audio.push(&pcm(LIVE_PAUSE_MS, 0));
        assert_eq!(
            step_name(next_live_step(&audio, Duration::ZERO, 0)),
            "finish"
        );
        audio.consume(audio.pcm.len());
        assert_eq!(
            step_name(next_live_step(&audio, LIVE_PASS_INTERVAL, 0)),
            "wait"
        );
    }

    #[test]
    fn stopping_finishes_speech_or_stops_on_silence() {
        let mut audio = LiveAudio::default();
        audio.push(&pcm(500, 8_000));
        audio.stopping = true;
        assert_eq!(
            step_name(next_live_step(&audio, Duration::ZERO, 0)),
            "finish"
        );
        let mut silent = LiveAudio::default();
        silent.push(&pcm(500, 0));
        silent.stopping = true;
        assert_eq!(
            step_name(next_live_step(&silent, Duration::ZERO, 0)),
            "stop"
        );
    }
}
