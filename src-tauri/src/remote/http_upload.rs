use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;

/// A single WAV recording extracted from a multipart file part.
pub(super) struct WavUpload(pub(super) Vec<u8>);

/// Pulls the WAV out of a multipart/form-data body.
///
/// The desktop client posts one file part, so the payload begins at the RIFF
/// magic. The declared RIFF size gives the exact end offset, which avoids
/// trusting the multipart boundary at the tail.
pub(super) fn wav_from_multipart(body: &[u8]) -> Option<WavUpload> {
    let start = body.windows(4).position(|window| window == b"RIFF")?;
    let size = u32::from_le_bytes([
        *body.get(start + 4)?,
        *body.get(start + 5)?,
        *body.get(start + 6)?,
        *body.get(start + 7)?,
    ]) as usize;
    let end = start.checked_add(size + 8)?;
    if end > body.len() {
        return None;
    }
    Some(WavUpload(body[start..end].to_vec()))
}

/// Longest live hint (prompt or hotwords) accepted, matching the local speech service.
const MAX_LIVE_HINT_CHARS: usize = 400;

/// Reads one short UTF-8 text part (`name="..."`) from a multipart body.
pub(super) fn multipart_text_field(body: &[u8], name: &str) -> Option<String> {
    let marker = format!("name=\"{name}\"");
    let find = |haystack: &[u8], needle: &[u8]| {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    };
    let after_name = find(body, marker.as_bytes())? + marker.len();
    let value_start = after_name + find(&body[after_name..], b"\r\n\r\n")? + 4;
    let value_end = value_start + find(&body[value_start..], b"\r\n--")?;
    String::from_utf8(body[value_start..value_end].to_vec()).ok()
}

/// Reads the declared body length from a request head.
pub(super) fn content_length(head: &str) -> Option<usize> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("content-length") {
            return None;
        }
        value.trim().parse().ok()
    })
}

/// Accepts the shared token as a bearer credential, which is the header the
/// desktop client already sends when it carries an API key.
pub(super) fn bearer_token(head: &str) -> Option<&str> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("authorization") {
            return None;
        }
        value.trim().strip_prefix("Bearer ")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn legacy_requests_keep_clean_and_accept_only_valid_optional_modes() {
        assert_eq!(
            request_polish_options("POST / HTTP/1.1\r\nHost: test\r\n").unwrap(),
            crate::stt::RemotePolishOptions::default()
        );
        for style in crate::storage::POLISH_STYLES {
            let head = format!("POST / HTTP/1.1\r\nX-Ultra-STT-Polish-Style: {style}\r\nx-ultra-stt-polish-enabled: false\r\n");
            assert_eq!(
                request_polish_options(&head).unwrap(),
                crate::stt::RemotePolishOptions {
                    style: style.to_string(),
                    enabled: false
                }
            );
        }
        for head in [
            "x-ultra-stt-polish-style: unknown",
            "x-ultra-stt-polish-enabled: maybe",
            "x-ultra-stt-polish-style: original\r\nx-ultra-stt-polish-style: clean",
            "x-ultra-stt-polish-enabled: true\r\nx-ultra-stt-polish-enabled: false",
        ] {
            assert!(request_polish_options(head).is_err());
        }
    }

    #[tokio::test]
    async fn disconnect_monitor_waits_for_a_live_client_and_detects_cancellation() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut server, _) = listener.accept().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), wait_for_disconnect(&mut server))
                .await
                .is_err()
        );
        client.shutdown().await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), wait_for_disconnect(&mut server))
            .await
            .unwrap();
    }

    fn wav() -> Vec<u8> {
        let mut bytes = vec![0u8; 44 + 8];
        bytes[0..4].copy_from_slice(b"RIFF");
        let riff_size = ((bytes.len() - 8) as u32).to_le_bytes();
        bytes[4..8].copy_from_slice(&riff_size);
        bytes[8..12].copy_from_slice(b"WAVE");
        bytes[40..44].copy_from_slice(&8u32.to_le_bytes());
        bytes
    }

    #[test]
    fn extracts_wav_from_a_multipart_body() {
        let audio = wav();
        let mut body = Vec::new();
        body.extend_from_slice(
            b"--b\r\nContent-Disposition: form-data; name=model\r\n\r\nlarge-v3\r\n",
        );
        body.extend_from_slice(b"--b\r\nContent-Disposition: form-data; name=file; filename=a.wav\r\nContent-Type: audio/wav\r\n\r\n");
        body.extend_from_slice(&audio);
        body.extend_from_slice(b"\r\n--b--\r\n");
        let extracted = wav_from_multipart(&body).expect("wav should be found");
        assert_eq!(extracted.0, audio);
    }

    #[test]
    fn rejects_a_body_without_wav_or_with_a_truncated_tail() {
        assert!(wav_from_multipart(b"not a multipart body at all").is_none());
        let mut body = Vec::new();
        body.extend_from_slice(b"RIFF");
        body.extend_from_slice(&9999u32.to_le_bytes());
        body.extend_from_slice(&[0u8; 16]);
        assert!(wav_from_multipart(&body).is_none());
    }

    #[test]
    fn reads_live_hint_fields_from_a_multipart_body() {
        let body = "--b\r\nContent-Disposition: form-data; name=\"prompt\"\r\n\r\n앞 문장\r\n--b\r\nContent-Disposition: form-data; name=\"hotwords\"\r\n\r\n슬관절, 비급여\r\n--b--\r\n";
        assert_eq!(
            multipart_text_field(body.as_bytes(), "prompt").as_deref(),
            Some("앞 문장")
        );
        assert_eq!(
            multipart_text_field(body.as_bytes(), "hotwords").as_deref(),
            Some("슬관절, 비급여")
        );
        assert_eq!(multipart_text_field(body.as_bytes(), "missing"), None);
    }

    #[test]
    fn reads_body_length_and_bearer_credential() {
        let head = "POST /v1/audio/transcriptions HTTP/1.1\r\nHost: x\r\nContent-Length: 1234\r\nAuthorization: Bearer secret-value\r\n\r\n";
        assert_eq!(content_length(head), Some(1234));
        assert_eq!(bearer_token(head), Some("secret-value"));
        assert_eq!(bearer_token("Authorization: Basic abc\r\n"), None);
        assert_eq!(content_length("POST / HTTP/1.1\r\n"), None);
    }
}

/// Answers one OpenAI-shaped transcription request on the loopback listener.
///
/// The Mac build posts here with the shared token as its API key, receives the
/// cleaned text, and types it locally, so no local Whisper is needed on macOS.
pub(super) async fn handle(
    mut stream: tokio::net::TcpStream,
    _owner: &str,
    _origin: &str,
) -> Result<()> {
    let mut buffer: Vec<u8> = Vec::with_capacity(16 * 1024);
    let mut chunk = [0u8; 16 * 1024];
    let head_end = loop {
        if let Some(position) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            break position;
        }
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            anyhow::bail!("client closed before sending a request head");
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > 64 * 1024 {
            anyhow::bail!("request head is unreasonably large");
        }
    };
    let head = String::from_utf8_lossy(&buffer[..head_end]).to_string();
    let expected = client_token();
    let presented = bearer_token(&head);
    let authorized = expected
        .as_deref()
        .zip(presented)
        .is_some_and(|(want, got)| constant_time_eq(want.as_bytes(), got.as_bytes()));
    if !authorized {
        return respond(
            &mut stream,
            403,
            "application/json",
            br#"{"error":"forbidden"}"#,
        )
        .await;
    }
    let options = match request_polish_options(&head) {
        Ok(options) => options,
        Err(_) => {
            return respond(
                &mut stream,
                400,
                "application/json",
                br#"{"error":"invalid polish options"}"#,
            )
            .await
        }
    };
    // Tailscale Serve strips the configured path prefix before forwarding, so the
    // same request arrives as /audio/transcriptions when it came through /v1.
    let is_transcription = head.starts_with("POST /v1/audio/transcriptions")
        || head.starts_with("POST /audio/transcriptions");
    // Live typing passes from other PCs: a short window recognised on this PC's model.
    let is_live = head.starts_with("POST /v1/audio/live ") || head.starts_with("POST /audio/live ");
    if !is_transcription && !is_live {
        return respond(
            &mut stream,
            404,
            "application/json",
            br#"{"error":"not found"}"#,
        )
        .await;
    }
    let length = content_length(&head).unwrap_or(0);
    if length == 0 || length > MAX_WAV {
        return respond(
            &mut stream,
            413,
            "application/json",
            br#"{"error":"audio too large"}"#,
        )
        .await;
    }
    let mut body = buffer.split_off(head_end + 4);
    while body.len() < length {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            anyhow::bail!("client closed mid upload");
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(length);
    let Some(wav) = wav_from_multipart(&body) else {
        return respond(
            &mut stream,
            400,
            "application/json",
            br#"{"error":"expected a wav file part"}"#,
        )
        .await;
    };
    let pcm = match pcm_from_wav(&wav.0) {
        Ok(pcm) => pcm.to_vec(),
        Err(_) => {
            return respond(
                &mut stream,
                400,
                "application/json",
                br#"{"error":"expected mono PCM16 16 kHz WAV"}"#,
            )
            .await;
        }
    };
    if is_live {
        let prompt = multipart_text_field(&body, "prompt").unwrap_or_default();
        let hotwords = multipart_text_field(&body, "hotwords").unwrap_or_default();
        if prompt.chars().count() > MAX_LIVE_HINT_CHARS
            || hotwords.chars().count() > MAX_LIVE_HINT_CHARS
        {
            return respond(
                &mut stream,
                400,
                "application/json",
                br#"{"error":"live hints too long"}"#,
            )
            .await;
        }
        let reply = match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            crate::stt::ultra_local::transcribe_live(&pcm, Some("ko"), &prompt, &hotwords),
        )
        .await
        {
            Ok(Ok(text)) => serde_json::json!({"text": text.unwrap_or_default()}),
            Ok(Err(error)) => {
                tracing::warn!("Remote live pass failed: {error}");
                serde_json::json!({"error": "live pass could not be processed"})
            }
            Err(_) => serde_json::json!({"error": "live pass timed out"}),
        };
        let payload = serde_json::to_vec(&reply).unwrap_or_default();
        return respond(&mut stream, 200, "application/json", &payload).await;
    }
    let work = tokio::time::timeout(
        crate::stt::processing_budget::remote(crate::stt::processing_budget::pcm_seconds(
            pcm.len(),
            16000,
        )),
        dictate_with_options(&pcm, &options),
    );
    let result = tokio::select! {
        result = work => result,
        _ = wait_for_disconnect(&mut stream) => {
            tracing::info!("Remote transcription cancelled by client disconnect");
            return Ok(());
        }
    };
    let reply = match result {
        Ok(Ok(payload)) => payload,
        Ok(Err(error)) => {
            tracing::warn!("Remote transcription failed: {error}");
            serde_json::json!({"error": "voice input could not be processed"})
        }
        Err(_) => serde_json::json!({"error": "voice input timed out"}),
    };
    let payload = serde_json::to_vec(&reply).unwrap_or_default();
    respond(&mut stream, 200, "application/json", &payload).await
}

fn request_polish_options(head: &str) -> std::result::Result<crate::stt::RemotePolishOptions, ()> {
    let mut options = crate::stt::RemotePolishOptions::default();
    let mut style_seen = false;
    let mut enabled_seen = false;
    for line in head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("x-ultra-stt-polish-style") {
            if style_seen || !crate::storage::POLISH_STYLES.contains(&value) {
                return Err(());
            }
            style_seen = true;
            options.style = value.to_string();
        } else if name.eq_ignore_ascii_case("x-ultra-stt-polish-enabled") {
            if enabled_seen {
                return Err(());
            }
            enabled_seen = true;
            options.enabled = match value {
                "true" => true,
                "false" => false,
                _ => return Err(()),
            };
        }
    }
    Ok(options)
}

async fn wait_for_disconnect(stream: &mut tokio::net::TcpStream) {
    let mut buffer = [0u8; 1024];
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}

async fn respond(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    content_type: &str,
    payload: &[u8],
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(payload).await?;
    stream.flush().await?;
    Ok(())
}
