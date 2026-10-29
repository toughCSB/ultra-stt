use crate::{credentials, engine_profile, storage};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResult {
    success: bool,
    message: String,
}

fn selection_valid(kind: &str, engine: &str, model: &str) -> bool {
    matches!(kind, "stt" | "llm")
        && matches!(engine, "existing" | "openai")
        && engine_profile::DESKTOP_MODELS.contains(&model)
}

async fn response_json(mut response: reqwest::Response) -> Result<serde_json::Value, String> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "응답을 읽지 못했습니다.")?
    {
        if bytes.len().saturating_add(chunk.len()) > 65_536 {
            return Err("연결 확인 응답이 너무 큽니다.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "연결 확인 응답이 올바르지 않습니다.".into())
}

async fn check_openai(config: &storage::AppConfig, model: &str) -> Result<String, String> {
    let key = credentials::resolve_stt_config_secret(config, &credentials::SystemCredentialVault)
        .map_err(|_| "저장된 OpenAI API 키를 읽지 못했습니다.")?;
    if key.trim().is_empty() {
        return Err("OpenAI API 키를 먼저 저장해 주세요.".into());
    }
    let client = engine_profile::openai_http_client()
        .map_err(|_| "HTTPS 클라이언트를 준비하지 못했습니다.")?;
    let response = client
        .get(format!("https://api.openai.com/v1/models/{model}"))
        .bearer_auth(key)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| "OpenAI에 연결하지 못했습니다. 인터넷 연결을 확인해 주세요.")?;
    if !response.status().is_success() {
        return Err(format!(
            "OpenAI 모델 조회 실패 (HTTP {}). 키·모델 조회 권한을 확인해 주세요.",
            response.status().as_u16()
        ));
    }
    let body = response_json(response).await?;
    if body["id"].as_str().is_none_or(str::is_empty) || body["object"] != "model" {
        return Err("OpenAI 모델 확인 응답이 올바르지 않습니다.".into());
    }
    Ok(format!(
        "연결 성공 · OpenAI 인증 및 {model} 조회 확인. 실제 전사·생성 요청은 실행하지 않았습니다."
    ))
}

async fn check_stt(config: &storage::AppConfig) -> Result<String, String> {
    if config.engine_mode == engine_profile::OPENAI_ENGINE {
        return check_openai(config, "gpt-transcribe").await;
    }
    if config.stt_provider != crate::stt::config::CUSTOM_WHISPER_PROVIDER {
        return Err("기존 Whisper 연결 설정을 확인해 주세요.".into());
    }
    if config.stt_custom_preset == "ultra-local" {
        tokio::time::timeout(Duration::from_secs(130), crate::stt::ultra_local::warm())
            .await
            .map_err(|_| "로컬 음성 엔진 준비 시간이 초과됐습니다.")?
            .map_err(|_| {
                "로컬 음성 엔진을 준비하지 못했습니다. 모델·GPU·서버 로그를 확인해 주세요."
            })?;
        return Ok(
            "연결 성공 · 로컬 Whisper 모델 준비 완료. 실제 음성 전사는 실행하지 않았습니다.".into(),
        );
    }
    if config.stt_custom_preset != crate::stt::config::ULTRA_REMOTE_STT_PRESET {
        return Err("기존 개인용 Whisper 서버 연결 설정이 아닙니다.".into());
    }
    let endpoint = crate::stt::config::build_custom_whisper_config(
        &config.stt_custom_base_url,
        &config.stt_custom_model,
    )
    .map_err(|_| "서버 주소가 올바르지 않습니다.")?
    .endpoint;
    let token = credentials::resolve_stt_config_secret(config, &credentials::SystemCredentialVault)
        .map_err(|_| "기존 서버 인증 정보를 읽지 못했습니다.")?;
    if token.trim().is_empty() {
        return Err("기존 서버 인증 정보가 없습니다.".into());
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "HTTPS 연결을 준비하지 못했습니다.")?;
    let response = client
        .post(endpoint)
        .bearer_auth(token)
        .body(Vec::<u8>::new())
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|_| "기존 서버에 연결하지 못했습니다. 서버 PC와 Tailscale을 확인해 주세요.")?;
    if response.status().as_u16() != 413 {
        return Err(format!(
            "기존 서버 연결·인증 확인 실패 (HTTP {}).",
            response.status().as_u16()
        ));
    }
    if response_json(response).await?["error"] != "audio too large" {
        return Err("기존 서버의 연결 확인 응답이 올바르지 않습니다.".into());
    }
    Ok("연결 성공 · 기존 서버 통신·인증 확인. 실제 전사와 GPU 속도는 검사하지 않았습니다.".into())
}

#[tauri::command]
pub async fn test_engine_setup(
    window: tauri::Window,
    state: tauri::State<'_, storage::ConfigManager>,
    kind: String,
    engine_mode: String,
    desktop_polish_model: String,
) -> Result<ConnectionTestResult, String> {
    if window.label() != "main" || !selection_valid(&kind, &engine_mode, &desktop_polish_model) {
        return Err("올바른 설정 화면에서 지원 모델을 선택해 주세요.".into());
    }
    let mut saved = state
        .load()
        .await
        .map_err(|_| "저장된 설정을 읽지 못했습니다.")?;
    saved.engine_mode = engine_mode;
    saved.desktop_polish_model = desktop_polish_model;
    let config = saved.effective_engine_config();
    let outcome = if kind == "stt" {
        check_stt(&config).await
    } else if config.llm_provider == "openai" {
        check_openai(&config, &config.llm_model).await
    } else {
        let prompt = "Connection test. Return only OK. Do not use tools or read files.";
        let response = if config.llm_provider == "codex-cli" {
            crate::llm::codex::run_text_with_effort(
                &config.llm_model,
                prompt,
                crate::llm::codex::ReasoningEffort::Low,
            )
            .await
        } else {
            crate::llm::cli::complete(
                &config.llm_model,
                prompt,
                crate::llm::codex::ReasoningEffort::Low,
            )
            .await
        };
        response
            .map(|_| {
                format!(
                    "연결 성공 · 이 컴퓨터의 {} 모델 응답 확인. CLI 사용량이 적용됩니다.",
                    config.llm_model
                )
            })
            .map_err(|error| format!("CLI 연결 실패: {error}"))
    };
    Ok(match outcome {
        Ok(message) => ConnectionTestResult {
            success: true,
            message,
        },
        Err(message) => ConnectionTestResult {
            success: false,
            message,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_two_engines_and_six_exact_models_can_be_probed() {
        for engine in ["existing", "openai"] {
            for model in engine_profile::DESKTOP_MODELS {
                assert!(selection_valid("stt", engine, model));
                assert!(selection_valid("llm", engine, model));
            }
        }
        assert!(!selection_valid("other", "openai", "gpt-6-luna"));
        assert!(!selection_valid("llm", "custom", "gpt-6-luna"));
        assert!(!selection_valid("llm", "openai", "../../../models"));
        assert!(!selection_valid("llm", "existing", "gpt-4o"));
    }
}
