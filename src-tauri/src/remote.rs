//! Personal tailnet input. Tailscale Serve authenticates the caller and proxies
//! WebSockets to this loopback-only listener; no cloud API credentials leave the PC.
use std::{path::PathBuf, process::Command, sync::Arc, time::Instant};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpListener, sync::Semaphore};
use tokio_tungstenite::{
    accept_hdr_async_with_config,
    tungstenite::{
        handshake::server::{Callback, ErrorResponse, Request, Response},
        http::StatusCode,
        protocol::{Message, WebSocketConfig},
    },
};

use crate::{
    app_detector::types::ContextProfile,
    llm::{self, LlmProvider, PolishRequest},
    stt::ultra_local,
    voice_intent::{VoiceIntent, VoiceIntentKind, VoiceOutputPlacement},
};

const ADDRESS: &str = "127.0.0.1:17846";
const HTTPS_PORT: u16 = 10443;
const MAX_PCM: usize = 24 * 1024 * 1024;
mod http_upload;

const MAX_WAV: usize = MAX_PCM + 44;

fn tailnet_identity(status: &serde_json::Value) -> Result<(String, String)> {
    anyhow::ensure!(
        status["BackendState"] == "Running",
        "Tailscale is not connected"
    );
    let id = status["Self"]["UserID"]
        .as_u64()
        .context("Tailscale owner is missing")?
        .to_string();
    let login = status["User"][&id]["LoginName"]
        .as_str()
        .filter(|value| !value.is_empty())
        .context("Tailscale owner login is missing")?;
    let dns = status["Self"]["DNSName"]
        .as_str()
        .map(|value| value.trim_end_matches('.'))
        .filter(|value| !value.is_empty())
        .context("Tailscale DNS name is missing")?;
    Ok((login.to_owned(), format!("https://{dns}:{HTTPS_PORT}")))
}

/// Path of the shared token that admits native clients such as the macOS build.
///
/// A browser reaches this listener through Tailscale Serve, which injects the
/// caller identity. A native app cannot forge those headers, so it presents a
/// shared token instead. The token lives in the personal data directory and is
/// copied to the client; when the file is absent, token auth stays closed.
pub const CLIENT_TOKEN_FILE: &str = "remote-client-token";

fn client_token() -> Option<String> {
    let root = std::env::var_os("ULTRA_STT_ROOT")?;
    let path = PathBuf::from(root).join("data").join(CLIENT_TOKEN_FILE);
    let token = std::fs::read_to_string(path).ok()?;
    let token = token.trim().to_string();
    (token.len() >= 32).then_some(token)
}

fn token_authorized(request: &Request) -> bool {
    let Some(expected) = client_token() else {
        return false;
    };
    request
        .headers()
        .get("x-ultra-stt-token")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| constant_time_eq(value.as_bytes(), expected.as_bytes()))
}

/// Length-independent comparison so a wrong token cannot be probed byte by byte.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

fn allowed(request: &Request, owner: &str, origin: &str) -> bool {
    let headers = request.headers();
    let identity = headers
        .get("tailscale-user-login")
        .and_then(|value| value.to_str().ok());
    let browser_origin = headers.get("origin");
    if !matches!(request.uri().path(), "/" | "/ws") {
        return false;
    }
    let browser = identity == Some(owner)
        && browser_origin.is_none_or(|value| value.to_str().ok() == Some(origin));
    browser || token_authorized(request)
}

struct AuthorizedClient {
    owner: String,
    origin: String,
}

impl Callback for AuthorizedClient {
    fn on_request(
        self,
        request: &Request,
        response: Response,
    ) -> std::result::Result<Response, ErrorResponse> {
        if allowed(request, &self.owner, &self.origin) {
            Ok(response)
        } else {
            let mut rejected = ErrorResponse::new(Some("Forbidden".to_string()));
            *rejected.status_mut() = StatusCode::FORBIDDEN;
            Err(rejected)
        }
    }
}

fn pcm_from_wav(wav: &[u8]) -> Result<&[u8]> {
    anyhow::ensure!(
        wav.len() >= 46 && wav.len() <= MAX_WAV,
        "WAV size is invalid"
    );
    let riff_size = u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]) as usize;
    let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as usize;
    anyhow::ensure!(
        &wav[0..4] == b"RIFF"
            && riff_size == wav.len() - 8
            && &wav[8..16] == b"WAVEfmt "
            && u32::from_le_bytes([wav[16], wav[17], wav[18], wav[19]]) == 16
            && u16::from_le_bytes([wav[20], wav[21]]) == 1
            && u16::from_le_bytes([wav[22], wav[23]]) == 1
            && u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]) == 16_000
            && u32::from_le_bytes([wav[28], wav[29], wav[30], wav[31]]) == 32_000
            && u16::from_le_bytes([wav[32], wav[33]]) == 2
            && u16::from_le_bytes([wav[34], wav[35]]) == 16
            && &wav[36..40] == b"data"
            && data_size == wav.len() - 44
            && data_size.is_multiple_of(2),
        "Expected mono PCM16 16 kHz WAV"
    );
    Ok(&wav[44..])
}

async fn dictate(pcm: &[u8]) -> Result<serde_json::Value> {
    dictate_with_options(pcm, &crate::stt::RemotePolishOptions::default()).await
}

fn polish_result_or_raw(
    raw: &str,
    result: std::result::Result<llm::PolishResponse, crate::error::AppError>,
) -> String {
    match result {
        Ok(response) if !response.polished_text.trim().is_empty() => response.polished_text,
        Ok(_) | Err(_) => {
            tracing::warn!(
                "Remote AI cleanup failed or returned empty text; preserving the raw transcript"
            );
            raw.to_string()
        }
    }
}

async fn dictate_with_options(
    pcm: &[u8],
    options: &crate::stt::RemotePolishOptions,
) -> Result<serde_json::Value> {
    let started = Instant::now();
    let raw = ultra_local::transcribe(pcm, Some("ko"))
        .await?
        .context("No speech was recognized")?;
    let stt_elapsed = started.elapsed();
    tracing::info!(
        stt_ms = stt_elapsed.as_millis() as u64,
        audio_ms = pcm.len() / 32,
        "Remote transcription completed"
    );
    let voice_intent = VoiceIntent::from_parts(
        VoiceIntentKind::DictateInsert,
        VoiceOutputPlacement::InsertAtCursor,
        1.0,
        None,
        None,
        None,
        None,
    )?;
    let request = PolishRequest {
        raw_text: raw.clone(),
        context: ContextProfile::general_native().summary(),
        dictionary: Vec::new(),
        correction_rules: Vec::new(),
        polish_style: options.style.clone(),
        mapped_scene_prompt: String::new(),
        active_scene_prompt: String::new(),
        polish_custom_prompt: String::new(),
        translate_enabled: false,
        target_lang: "ko".to_string(),
        selected_text: None,
        operation_id: None,
        voice_intent,
    };
    let config = llm::LlmConfig {
        provider: "codex-cli".to_string(),
        model: "gpt-6-luna".to_string(),
        ..Default::default()
    };
    let polished = if options.enabled {
        polish_result_or_raw(
            &raw,
            llm::codex::CodexProvider
                .polish(&config, &request, None)
                .await,
        )
    } else {
        raw.clone()
    };
    let total_elapsed = started.elapsed();
    tracing::info!(
        stt_ms = stt_elapsed.as_millis() as u64,
        llm_ms = total_elapsed.saturating_sub(stt_elapsed).as_millis() as u64,
        total_ms = total_elapsed.as_millis() as u64,
        polish_style = options.style.as_str(),
        polish_enabled = options.enabled,
        "Remote dictation completed"
    );
    Ok(serde_json::json!({"type":"result","raw":raw,"text":polished}))
}

async fn acknowledge_close(
    websocket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
) -> Result<()> {
    match websocket.flush().await {
        Ok(()) | Err(tokio_tungstenite::tungstenite::Error::ConnectionClosed) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn handle(stream: tokio::net::TcpStream, owner: String, origin: String) -> Result<()> {
    let mut peeked = [0u8; 4];
    // The first four bytes may not have arrived yet, and a short peek would
    // misroute a POST into the WebSocket handshake, so wait for all four.
    let mut filled = 0usize;
    while filled < 4 {
        match stream.peek(&mut peeked[filled..]).await {
            Ok(0) | Err(_) => break,
            Ok(read) => filled += read,
        }
    }
    if &peeked == b"POST" {
        return http_upload::handle(stream, &owner, &origin).await;
    }
    let websocket = accept_hdr_async_with_config(
        stream,
        AuthorizedClient { owner, origin },
        Some(WebSocketConfig {
            max_message_size: Some(MAX_WAV),
            max_frame_size: Some(MAX_WAV),
            ..Default::default()
        }),
    )
    .await?;
    let mut websocket = websocket;
    while let Some(message) = websocket.next().await {
        match message? {
            Message::Binary(wav) => {
                let pcm = match pcm_from_wav(&wav) {
                    Ok(pcm) => pcm,
                    Err(_) => {
                        websocket
                            .send(Message::Text(
                                r#"{"type":"error","message":"Expected mono PCM16 16 kHz WAV"}"#
                                    .to_string(),
                            ))
                            .await?;
                        continue;
                    }
                };
                let work = tokio::time::timeout(
                    crate::stt::processing_budget::remote(
                        crate::stt::processing_budget::pcm_seconds(pcm.len(), 16000),
                    ),
                    dictate(pcm),
                );
                tokio::pin!(work);
                loop {
                    tokio::select! {
                        outcome = &mut work => {
                            let result = match outcome {
                                Ok(Ok(result)) => result,
                                Ok(Err(error)) => {
                                    tracing::warn!("Remote dictation failed: {error}");
                                    serde_json::json!({"type":"error","message":"Voice input could not be processed"})
                                }
                                Err(_) => serde_json::json!({"type":"error","message":"Voice input timed out"}),
                            };
                            websocket.send(Message::Text(result.to_string())).await?;
                            break;
                        }
                        incoming = websocket.next() => {
                            match incoming {
                                Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                                Some(Ok(Message::Text(text))) if text == "cancel" => {
                                    websocket.close(None).await?;
                                    return Ok(()); // Dropping work cancels the owned model/CLI processes.
                                }
                                Some(Ok(Message::Close(_))) => {
                                    acknowledge_close(&mut websocket).await?;
                                    return Ok(());
                                }
                                _ => return Ok(()),
                            }
                        }
                    }
                }
            }
            Message::Close(_) => {
                acknowledge_close(&mut websocket).await?;
                return Ok(());
            }
            Message::Ping(_) | Message::Pong(_) => {}
            _ => {
                websocket
                    .send(Message::Text(
                        r#"{"type":"error","message":"Send one WAV recording"}"#.to_string(),
                    ))
                    .await?
            }
        }
    }
    Ok(())
}

pub async fn serve() -> Result<()> {
    let output = Command::new("C:/Program Files/Tailscale/tailscale.exe")
        .args(["status", "--json"])
        .output()
        .context("Tailscale is unavailable")?;
    anyhow::ensure!(output.status.success(), "Tailscale is not connected");
    let status: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let (owner, origin) = tailnet_identity(&status)?;
    let listener = TcpListener::bind(ADDRESS).await?;
    let slots = Arc::new(Semaphore::new(4));
    tracing::info!("Remote input listening on {ADDRESS}");
    loop {
        let (stream, _) = listener.accept().await?;
        let permit = slots.clone().acquire_owned().await?;
        let owner = owner.clone();
        let origin = origin.clone();
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(error) = handle(stream, owner, origin).await {
                tracing::debug!("Remote client disconnected: {error}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::http::Request as HttpRequest;

    #[test]
    fn cleanup_failure_preserves_the_complete_transcript() {
        let raw = "긴 녹음 원문입니다. ".repeat(300);
        assert_eq!(
            polish_result_or_raw(
                &raw,
                Err(crate::error::AppError::Timeout(Duration::from_secs(60)))
            ),
            raw
        );
        assert_eq!(
            polish_result_or_raw(
                &raw,
                Ok(llm::PolishResponse {
                    polished_text: "  ".into()
                })
            ),
            raw
        );
        assert_eq!(
            polish_result_or_raw(
                &raw,
                Ok(llm::PolishResponse {
                    polished_text: "정리된 결과".into()
                })
            ),
            "정리된 결과"
        );
    }

    #[test]
    fn tailnet_identity_requires_the_running_owner_and_dns_name() {
        let status = serde_json::json!({
            "BackendState":"Running",
            "Self":{"UserID":7,"DNSName":"server.example.ts.net."},
            "User":{"7":{"LoginName":"owner@example.test"}}
        });
        assert_eq!(
            tailnet_identity(&status).unwrap(),
            (
                "owner@example.test".to_string(),
                "https://server.example.ts.net:10443".to_string()
            )
        );
        assert!(tailnet_identity(&serde_json::json!({"BackendState":"Stopped"})).is_err());
    }

    #[test]
    fn shared_token_auth_opens_only_for_the_right_token() {
        // One test on purpose: these cases mutate ULTRA_STT_ROOT, and the test
        // harness runs tests in parallel threads, so splitting them races.
        let root = std::env::temp_dir().join(format!("ultra-remote-token-{}", std::process::id()));
        let data = root.join("data");
        std::fs::create_dir_all(&data).expect("temp data dir");
        // SAFETY: single test mutating the environment for the whole process run.
        unsafe { std::env::set_var("ULTRA_STT_ROOT", &root) };

        let owner = "owner@example.test";
        let origin = "https://server.example.ts.net:10443";
        let with = |value: &str| {
            HttpRequest::builder()
                .uri("/ws")
                .header("x-ultra-stt-token", value)
                .body(())
                .unwrap()
        };

        // No token file yet, so even a plausible header is refused.
        assert!(!allowed(&with("a".repeat(48).as_str()), owner, origin));

        let token = "a".repeat(48);
        std::fs::write(data.join(CLIENT_TOKEN_FILE), &token).expect("write token");
        assert!(allowed(&with(&token), owner, origin));
        assert!(!allowed(&with(&"b".repeat(48)), owner, origin));
        assert!(!allowed(&with(&"short"), owner, origin));

        std::fs::remove_dir_all(&root).ok();
        // SAFETY: restoring the previous state for any later test.
        unsafe { std::env::remove_var("ULTRA_STT_ROOT") };
    }
    #[test]
    fn websocket_rejects_other_users_and_cross_origin_browsers() {
        let make = |identity: Option<&str>, origin: Option<&str>| {
            let mut builder = HttpRequest::builder().uri("/ws");
            if let Some(identity) = identity {
                builder = builder.header("Tailscale-User-Login", identity);
            }
            if let Some(origin) = origin {
                builder = builder.header("Origin", origin);
            }
            builder.body(()).unwrap()
        };
        let owner = "owner@example.test";
        let origin = "https://server.example.ts.net:10443";
        assert!(allowed(&make(Some(owner), Some(origin)), owner, origin));
        assert!(allowed(&make(Some(owner), None), owner, origin));
        assert!(!allowed(&make(None, Some(origin)), owner, origin));
        assert!(!allowed(
            &make(Some("stranger@example.test"), None),
            owner,
            origin
        ));
        assert!(!allowed(
            &make(Some(owner), Some("https://evil.example")),
            owner,
            origin
        ));
        let non_utf8 = HttpRequest::builder()
            .uri("/ws")
            .header("Tailscale-User-Login", owner)
            .header(
                "Origin",
                tokio_tungstenite::tungstenite::http::HeaderValue::from_bytes(&[0xff]).unwrap(),
            )
            .body(())
            .unwrap();
        assert!(!allowed(&non_utf8, owner, origin));
    }

    #[test]
    fn wav_boundary_only_accepts_bounded_canonical_pcm() {
        let pcm = [0_i16, 100, -100, 0]
            .into_iter()
            .flat_map(i16::to_le_bytes)
            .collect::<Vec<_>>();
        let wav = crate::stt::whisper_compat::WhisperCompatProvider::build_wav(&pcm, 16_000);
        assert_eq!(pcm_from_wav(&wav).unwrap(), pcm);
        let mut invalid = wav.clone();
        invalid[24] = 0;
        assert!(pcm_from_wav(&invalid).is_err());
        assert!(pcm_from_wav(&wav[..42]).is_err());
    }

    #[tokio::test]
    async fn authenticated_socket_rejects_invalid_audio_without_loading_the_model() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle(
                stream,
                "owner@example.test".to_string(),
                "https://server.example.ts.net:10443".to_string(),
            )
            .await
        });
        let mut request = format!("ws://{address}/ws").into_client_request().unwrap();
        request.headers_mut().insert(
            "Tailscale-User-Login",
            "owner@example.test".parse().unwrap(),
        );
        request.headers_mut().insert(
            "Origin",
            "https://server.example.ts.net:10443".parse().unwrap(),
        );
        let (mut websocket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        websocket.send(Message::Binary(vec![0; 46])).await.unwrap();
        let response = tokio::time::timeout(Duration::from_secs(2), websocket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let payload: serde_json::Value = serde_json::from_str(response.to_text().unwrap()).unwrap();
        assert_eq!(payload["type"], "error");
        websocket.close(None).await.unwrap();
        let closing = tokio::time::timeout(Duration::from_secs(2), websocket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(matches!(closing, Message::Close(_)));
        server.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn socket_without_tailnet_identity_is_denied_before_audio() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            handle(
                stream,
                "owner@example.test".to_string(),
                "https://server.example.ts.net:10443".to_string(),
            )
            .await
        });
        let request = format!("ws://{address}/ws").into_client_request().unwrap();
        let error = tokio_tungstenite::connect_async(request).await.unwrap_err();
        assert!(matches!(
            error,
            tokio_tungstenite::tungstenite::Error::Http(response)
                if response.status() == StatusCode::FORBIDDEN
        ));
        assert!(server.await.unwrap().is_err());
    }
}
