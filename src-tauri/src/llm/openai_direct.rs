use super::{ChunkCallback, LlmConfig, PolishRequest, PolishResponse};
use crate::error::AppError;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;

const ENDPOINT: &str = "https://api.openai.com/v1/responses";
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub(super) fn matches(config: &LlmConfig) -> bool {
    config.provider == "openai"
        && crate::engine_profile::is_openai_model(&config.model)
        && config.base_url.trim_end_matches('/') == "https://api.openai.com/v1"
}

fn output_limit(config: &LlmConfig, request: &PolishRequest) -> usize {
    request
        .raw_text
        .chars()
        .count()
        .saturating_add(
            request
                .selected_text
                .as_deref()
                .unwrap_or_default()
                .chars()
                .count(),
        )
        .saturating_mul(2)
        .saturating_add(2048)
        .max(config.max_tokens as usize)
        .clamp(4096, 32768)
}

fn request_body(config: &LlmConfig, request: &PolishRequest, messages: Vec<Value>) -> Value {
    json!({
        "model": config.model,
        "input": messages,
        "reasoning": {"effort": super::codex::polish_effort(request).api_value()},
        "service_tier": "priority",
        "store": false,
        "max_output_tokens": output_limit(config, request)
    })
}

fn completed_text(response: &Value) -> Result<String, AppError> {
    if response["status"] != "completed"
        || response.get("error").is_some_and(|value| !value.is_null())
        || response
            .get("incomplete_details")
            .is_some_and(|value| !value.is_null())
    {
        return Err(AppError::Config(
            "OpenAI did not complete AI polishing; transcription is preserved".into(),
        ));
    }
    let mut text = String::new();
    for item in response["output"].as_array().into_iter().flatten() {
        if item["type"] != "message" || item["role"] != "assistant" {
            continue;
        }
        if item["status"] != "completed" {
            return Err(AppError::Config(
                "OpenAI returned an incomplete message".into(),
            ));
        }
        for part in item["content"].as_array().into_iter().flatten() {
            if part["type"] == "refusal" {
                return Err(AppError::Config(
                    "OpenAI could not polish this transcription".into(),
                ));
            }
            if part["type"] == "output_text" {
                if let Some(content) = part["text"].as_str() {
                    text.push_str(content);
                }
            }
        }
    }
    if text.trim().is_empty() {
        return Err(AppError::Config(
            "OpenAI returned no completed polishing text".into(),
        ));
    }
    Ok(text.trim().to_string())
}

pub(super) async fn polish(
    _client: &reqwest::Client,
    config: &LlmConfig,
    request: &PolishRequest,
    messages: Vec<Value>,
    on_chunk: Option<&ChunkCallback>,
) -> Result<PolishResponse, AppError> {
    let polished_text = complete(&config.api_key, &request_body(config, request, messages)).await?;
    if let Some(callback) = on_chunk {
        callback(&polished_text);
    }
    Ok(PolishResponse { polished_text })
}

pub(crate) async fn complete(api_key: &str, body: &Value) -> Result<String, AppError> {
    complete_at(
        &crate::engine_profile::openai_http_client()?,
        ENDPOINT,
        api_key,
        body,
    )
    .await
}

async fn complete_at(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    body: &Value,
) -> Result<String, AppError> {
    if api_key.trim().is_empty() {
        return Err(AppError::Config(
            "OpenAI API key is not configured in Engine Selection".into(),
        ));
    }
    let response = client
        .post(endpoint)
        .bearer_auth(api_key.trim())
        .json(body)
        .timeout(Duration::from_secs(60))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(AppError::Api {
            status: response.status().as_u16(),
            body: "OpenAI AI polishing request failed; transcription is preserved".into(),
        });
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(AppError::Config(
                "OpenAI polishing response exceeded its size limit".into(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Config("OpenAI returned invalid polishing JSON".into()))?;
    completed_text(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dictation_request() -> PolishRequest {
        PolishRequest {
            raw_text: "회의를 취소하지 마세요.".into(),
            context: crate::app_detector::types::ContextProfile::general_native().summary(),
            dictionary: vec![],
            correction_rules: vec![],
            polish_style: "original".into(),
            mapped_scene_prompt: String::new(),
            active_scene_prompt: String::new(),
            polish_custom_prompt: String::new(),
            translate_enabled: false,
            target_lang: "ko".into(),
            selected_text: None,
            operation_id: None,
            voice_intent: crate::voice_intent::VoiceIntent::from_parts(
                crate::voice_intent::VoiceIntentKind::DictateInsert,
                crate::voice_intent::VoiceOutputPlacement::InsertAtCursor,
                1.0,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        }
    }

    #[test]
    fn request_keeps_effort_policy_and_does_not_store_audio_or_key_in_body() {
        let config = LlmConfig {
            model: "gpt-6-luna".into(),
            ..LlmConfig::default()
        };
        let mut request = dictation_request();
        let body = request_body(
            &config,
            &request,
            vec![json!({"role":"user","content":request.raw_text})],
        );
        assert_eq!(body["model"], "gpt-6-luna");
        assert_eq!(body["reasoning"]["effort"], "low");
        assert_eq!(body["service_tier"], "priority");
        assert_eq!(body["store"], false);
        assert!(body.get("api_key").is_none());
        assert!(body.get("temperature").is_none());
        assert!(body.get("tools").is_none());
        request.polish_style = "structured".into();
        assert_eq!(
            request_body(&config, &request, vec![])["reasoning"]["effort"],
            "medium"
        );
        request.raw_text = "긴 문장을 보존합니다. ".repeat(1000);
        assert!(output_limit(&config, &request) > 4096);
        assert!(output_limit(&config, &request) <= 32768);
    }

    #[tokio::test]
    async fn real_http_path_validates_completed_response() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/v1/responses", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0; 8192];
            let received = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..received]);
            assert!(request.starts_with("POST /v1/responses"));
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-api-key"));
            let body = response("원문을 모두 보존했습니다.").to_string();
            let reply = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(reply.as_bytes()).await.unwrap();
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let text = complete_at(
            &client,
            &endpoint,
            "test-api-key",
            &json!({"model":"gpt-6-luna"}),
        )
        .await
        .unwrap();
        assert_eq!(text, "원문을 모두 보존했습니다.");
        task.await.unwrap();
    }

    fn response(text: &str) -> Value {
        json!({"status":"completed","error":null,"output":[
            {"type":"reasoning","summary":[]},
            {"type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":text}]}
        ]})
    }

    #[test]
    fn reads_assistant_output_after_reasoning_item() {
        assert_eq!(
            completed_text(&response("회의를 취소하지 마세요.")).unwrap(),
            "회의를 취소하지 마세요."
        );
    }

    #[test]
    fn rejects_partial_output_instead_of_silently_accepting_a_prefix() {
        let mut truncated = response("문장 앞부분만");
        truncated["status"] = json!("incomplete");
        truncated["incomplete_details"] = json!({"reason":"max_output_tokens"});
        assert!(completed_text(&truncated).is_err());
        let mut message = response("문장 앞부분만");
        message["output"][1]["status"] = json!("incomplete");
        assert!(completed_text(&message).is_err());
    }

    #[test]
    fn rejects_refusal_and_empty_results() {
        assert!(completed_text(&response(" ")).is_err());
        let mut refused = response("ignored");
        refused["output"][1]["content"] = json!([{"type":"refusal","refusal":"refused"}]);
        assert!(completed_text(&refused).is_err());
    }

    #[test]
    fn does_not_redirect_other_models_or_custom_hosts() {
        let mut config = LlmConfig {
            provider: "openai".into(),
            model: "gpt-6-luna".into(),
            base_url: "https://api.openai.com/v1".into(),
            ..LlmConfig::default()
        };
        assert!(matches(&config));
        config.base_url = "https://other.example/v1".into();
        assert!(!matches(&config));
        config.base_url = "https://api.openai.com/v1".into();
        config.model = "different-model".into();
        assert!(!matches(&config));
    }
}
