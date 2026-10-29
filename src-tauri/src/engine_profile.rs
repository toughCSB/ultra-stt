use crate::storage::AppConfig;
use std::{sync::OnceLock, time::Duration};

pub const OPENAI_ENGINE: &str = "openai";
pub const OPENAI_CREDENTIAL: &str = "openai-transcribe";
pub const OPENAI_LLM_MODEL: &str = "gpt-6-luna";
pub const DESKTOP_MODELS: &[&str] = &[
    "gpt-6.1-sol",
    "gpt-6.1-terra",
    "gpt-6-luna",
    "claude-haiku-5-5",
    "claude-sonnet-5-5",
    "grok-4.7",
];
static OPENAI_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

pub fn is_openai_model(model: &str) -> bool {
    matches!(model, "gpt-6.1-sol" | "gpt-6.1-terra" | "gpt-6-luna")
}

pub(crate) fn openai_http_client() -> Result<reqwest::Client, crate::error::AppError> {
    if let Some(client) = OPENAI_CLIENT.get() {
        return Ok(client.clone());
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .pool_max_idle_per_host(2)
        .pool_idle_timeout(Duration::from_secs(600))
        .tcp_keepalive(Duration::from_secs(60))
        .build()?;
    let _ = OPENAI_CLIENT.set(client.clone());
    Ok(client)
}

impl AppConfig {
    pub fn effective_engine_config(&self) -> Self {
        let mut active = self.clone();
        let model = if DESKTOP_MODELS.contains(&self.desktop_polish_model.as_str()) {
            self.desktop_polish_model.as_str()
        } else {
            OPENAI_LLM_MODEL
        };
        active.llm_model = model.to_string();
        active.llm_api_key.clear();
        active.llm_base_url.clear();
        active.llm_provider = match model {
            "claude-haiku-5-5" | "claude-sonnet-5-5" => "claude-cli",
            "grok-4.7" => "grok-cli",
            _ if self.engine_mode == OPENAI_ENGINE => "openai",
            _ => "codex-cli",
        }
        .into();
        if active.llm_provider == "openai" {
            active.llm_base_url = "https://api.openai.com/v1".into();
        }
        if self.engine_mode == OPENAI_ENGINE {
            active.stt_provider = OPENAI_CREDENTIAL.to_string();
            active.stt_api_key.clear();
            active.stt_custom_api_key.clear();
            active.stt_custom_preset.clear();
            active.stt_custom_base_url.clear();
            active.stt_custom_model.clear();
        }
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_configuration_defaults_to_existing_engine() {
        let config: AppConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.engine_mode, "existing");
        assert_eq!(config.desktop_polish_model, OPENAI_LLM_MODEL);
        assert_eq!(
            config.effective_engine_config().stt_provider,
            config.stt_provider
        );
        assert_eq!(config.effective_engine_config().llm_provider, "codex-cli");
    }

    #[test]
    fn direct_engine_preserves_saved_setup_and_dictation_preferences() {
        let mut saved = AppConfig::new_ultra_local_profile();
        saved.stt_custom_preset = "ultra-remote".into();
        saved.stt_custom_base_url = "https://example.test/v1".into();
        saved.stt_api_key = "legacy-server-secret".into();
        saved.stt_custom_api_key = "legacy-custom-secret".into();
        saved.llm_api_key = "legacy-llm-secret".into();
        saved.polish_style = "original".into();
        saved.engine_mode = OPENAI_ENGINE.into();
        let before = serde_json::to_value(&saved).unwrap();
        let active = saved.effective_engine_config();
        assert_eq!(serde_json::to_value(&saved).unwrap(), before);
        assert_eq!(active.stt_provider, OPENAI_CREDENTIAL);
        assert_eq!(active.llm_model, OPENAI_LLM_MODEL);
        assert_eq!(active.llm_provider, "openai");
        assert_eq!(active.llm_base_url, "https://api.openai.com/v1");
        assert!(active.stt_api_key.is_empty());
        assert!(active.stt_custom_api_key.is_empty());
        assert!(active.llm_api_key.is_empty());
        assert!(active.stt_custom_preset.is_empty());
        assert_eq!(active.polish_style, saved.polish_style);
        assert_eq!(active.hotkeys, saved.hotkeys);
        assert_eq!(active.max_recording_seconds, saved.max_recording_seconds);
        saved.engine_mode = "existing".into();
        let restored = saved.effective_engine_config();
        assert_eq!(restored.stt_custom_base_url, "https://example.test/v1");
        assert_eq!(restored.llm_provider, "codex-cli");
    }

    #[test]
    fn six_models_use_explicit_api_or_local_cli_routes_without_overwriting_saved_settings() {
        for engine in ["existing", "openai"] {
            for model in DESKTOP_MODELS {
                let mut config = AppConfig::new_ultra_local_profile();
                config.engine_mode = engine.into();
                config.desktop_polish_model = (*model).into();
                let before = serde_json::to_value(&config).unwrap();
                let active = config.effective_engine_config();
                assert_eq!(serde_json::to_value(&config).unwrap(), before);
                assert_eq!(active.llm_model, *model);
                let provider = match *model {
                    "claude-haiku-5-5" | "claude-sonnet-5-5" => "claude-cli",
                    "grok-4.7" => "grok-cli",
                    _ if engine == "openai" => "openai",
                    _ => "codex-cli",
                };
                assert_eq!(active.llm_provider, provider);
                assert!(active.llm_api_key.is_empty());
            }
        }
    }
}
