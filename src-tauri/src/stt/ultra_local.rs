//! Owned project-local CUDA speech service. Taking the owner out of the warm slot
//! ensures dropping a cancelled inference closes the Job instead of retaining GPU work.
use std::{sync::OnceLock, time::{Duration, Instant}};
use async_trait::async_trait;
use tokio::sync::Mutex;
use crate::{error::AppError, local_process::{Limits, LocalProcess}};
use super::{SttConfig, SttProvider, TranscriptEvent, whisper_compat::WhisperCompatProvider};

pub const BASE: &str = "http://127.0.0.1:17845/v1";
pub const MODEL: &str = "Systran/faster-whisper-large-v3";
const ENDPOINT: &str = "http://127.0.0.1:17845/v1/audio/transcriptions";
const MAX_PCM: usize = 24 * 1024 * 1024;
struct Service { owner: LocalProcess, secret: String }
static SERVICE: OnceLock<Mutex<Option<Service>>> = OnceLock::new();
static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn service() -> &'static Mutex<Option<Service>> { SERVICE.get_or_init(|| Mutex::new(None)) }
fn error(detail: impl std::fmt::Display) -> AppError { AppError::Config(format!("Local speech service: {detail}")) }
fn client() -> Result<reqwest::Client, AppError> {
    if let Some(client) = CLIENT.get() { return Ok(client.clone()); }
    let client = reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(60)).build().map_err(error)?;
    let _ = CLIENT.set(client.clone());
    Ok(client)
}
fn root() -> Result<std::path::PathBuf, AppError> {
    let path = std::env::var_os("ULTRA_STT_ROOT").ok_or_else(|| error("ULTRA_STT_ROOT is unset; launch via start-ultra-stt.ps1"))?;
    let path = std::path::PathBuf::from(path);
    if !path.is_absolute() { return Err(error("ULTRA_STT_ROOT must be absolute")); }
    Ok(path)
}
async fn start() -> Result<Service, AppError> {
    let started = Instant::now();
    let root = root()?;
    let interpreter = root.join(".venv/Scripts/python.exe");
    let app = root.join("opentypeless");
    let secret = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let limits = Limits { stdin: 256, stdout: 512, stderr: 16 * 1024, deadline: Duration::from_secs(120) };
    let mut owner = LocalProcess::spawn(interpreter, limits, |command| {
        command.args(["-m", "local_stt.server"]).current_dir(&app)
            .env("ULTRA_STT_ROOT", &root).env("HF_HUB_OFFLINE", "1");
    }).map_err(error)?;
    let bootstrap = serde_json::json!({"protocol":1,"session_secret":secret}).to_string() + "\n";
    let deadline = Instant::now() + limits.deadline;
    owner.send_stdin(bootstrap.as_bytes(), limits.stdin, limits.deadline).await.map_err(error)?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    let line = owner.ready_line(limits.stdout, remaining).await.map_err(error)?;
    let ready: serde_json::Value = serde_json::from_slice(&line).map_err(error)?;
    if ready["event"] != "ready" || ready["protocol"] != 1 || ready["endpoint"] != BASE
        || ready["pid"].as_u64().is_none_or(|pid| pid == 0) {
        return Err(error("invalid helper readiness event"));
    }
    tracing::info!(startup_ms = started.elapsed().as_millis() as u64, "Local speech service ready");
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
    if pcm.len() > MAX_PCM || !pcm.len().is_multiple_of(2) { return Err(error("invalid PCM length")); }
    if pcm.is_empty() { return Ok(None); }
    let mut slot = service().lock().await;
    let owned = match slot.take() { Some(owned) => owned, None => start().await? };
    // If this future is cancelled, `owned` is dropped and its Job kills all descendants.
    let wav = WhisperCompatProvider::build_wav(pcm, 16000);
    let file = reqwest::multipart::Part::bytes(wav).file_name("audio.wav")
        .mime_str("audio/wav").map_err(error)?;
    let mut form = reqwest::multipart::Form::new().text("model", MODEL).part("file", file);
    if let Some(lang) = language.filter(|lang| *lang != "multi") { form = form.text("language", lang.to_string()); }
    let request_start = Instant::now();
    let response = client()?.post(ENDPOINT).header("Host", "127.0.0.1:17845")
        .header("X-Ultra-STT-Session", &owned.secret).multipart(form).send().await.map_err(error)?;
    if !response.status().is_success() { return Err(error(format!("HTTP {}", response.status()))); }
    let payload: serde_json::Value = response.json().await.map_err(error)?;
    let text = payload["text"].as_str().ok_or_else(|| error("missing transcription"))?.trim().to_owned();
    tracing::info!(request_ms = request_start.elapsed().as_millis() as u64, audio_ms = pcm.len() / 32, "Local speech transcription completed");
    *slot = Some(owned); // successful inference retains the ready CUDA model
    Ok((!text.is_empty()).then_some(text))
}

#[derive(Default)]
pub struct UltraLocalProvider { buffer: Vec<u8>, config: Option<SttConfig> }
impl UltraLocalProvider { pub fn new() -> Self { Self::default() } }
#[async_trait]
impl SttProvider for UltraLocalProvider {
    async fn connect(&mut self, config: &SttConfig) -> Result<(), AppError> {
        if config.sample_rate != 16000 { return Err(error("16 kHz PCM is required")); }
        let mut slot = service().lock().await;
        if slot.is_none() { *slot = Some(start().await?); }
        self.config = Some(config.clone());
        self.buffer.clear();
        Ok(())
    }
    async fn send_audio(&mut self, chunk: &[u8]) -> Result<(), AppError> {
        if self.buffer.len().saturating_add(chunk.len()) > MAX_PCM { return Err(error("PCM limit exceeded")); }
        self.buffer.extend_from_slice(chunk);
        Ok(())
    }
    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>, AppError> { std::future::pending().await }
    async fn disconnect(&mut self) -> Result<Option<String>, AppError> {
        let config = self.config.take();
        let pcm = std::mem::take(&mut self.buffer);
        transcribe(&pcm, config.as_ref().and_then(|config| config.language.as_deref())).await
    }
    fn name(&self) -> &str { "custom-whisper" }
}

pub async fn shutdown() {
    let mut slot = service().lock().await;
    if let Some(mut owned) = slot.take() { let _ = owned.owner.kill(Duration::from_secs(5)).await; }
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
                connection.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK").await.unwrap();
            }
        });
        let requests = async {
            for _request in 0..2 {
                let response = client().unwrap().get(&endpoint).send().await.unwrap();
                assert_eq!(response.bytes().await.unwrap().as_ref(), b"OK");
            }
            server.await.unwrap();
        };
        tokio::time::timeout(Duration::from_secs(3), requests).await.unwrap();
    }

    #[test]
    fn local_endpoint_is_exact() {
        assert_eq!(ENDPOINT, format!("{BASE}/audio/transcriptions"));
        assert_eq!(MODEL, "Systran/faster-whisper-large-v3");
    }
}
