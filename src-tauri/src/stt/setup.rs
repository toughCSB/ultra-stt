//! Decides how one recording is transcribed: which key it uses, whether that is enough,
//! and the provider settings that go with it.

use super::config::{
    build_custom_whisper_for_app, stt_provider_requires_api_key, CUSTOM_WHISPER_PROVIDER,
};
use super::whisper_compat::WhisperCompatConfig;
use super::SttConfig;
use crate::storage::AppConfig;

/// Where speech recognition keys come from.
pub trait SttSecrets {
    /// Signed-in cloud session token; empty when signed out.
    fn cloud_session_token(&self) -> String;
    /// The provider key kept in the system credential vault.
    fn stt_secret(&self, config: &AppConfig) -> anyhow::Result<String>;
}

/// Everything needed to build and connect the recognizer for one recording.
pub struct SttSetup {
    pub stt_config: SttConfig,
    /// Set for the Local / Custom Whisper provider.
    pub custom_whisper: Option<WhisperCompatConfig>,
}

#[derive(Debug)]
pub enum SttSetupError {
    /// The vault could not be read.
    SecretUnavailable(anyhow::Error),
    /// The provider needs a key and none is configured.
    MissingApiKey,
    /// The Local / Custom Whisper settings are not usable.
    InvalidCustomWhisper(String),
}

/// The local Whisper preset runs on this PC and takes no key.
const ULTRA_LOCAL_PRESET: &str = "ultra-local";

/// Resolves the recognizer settings for `config`. `operation_id` tags cloud requests.
pub fn resolve_stt_setup(
    config: &AppConfig,
    secrets: &dyn SttSecrets,
    operation_id: Option<String>,
    now_unix_seconds: i64,
) -> Result<SttSetup, SttSetupError> {
    let api_key = if config.stt_provider == CUSTOM_WHISPER_PROVIDER
        && config.stt_custom_preset == ULTRA_LOCAL_PRESET
    {
        String::new()
    } else if config.stt_provider == "cloud" {
        secrets.cloud_session_token()
    } else {
        secrets
            .stt_secret(config)
            .map_err(SttSetupError::SecretUnavailable)?
    };
    if stt_provider_requires_api_key(&config.stt_provider) && api_key.is_empty() {
        return Err(SttSetupError::MissingApiKey);
    }
    let custom_whisper = if config.stt_provider == CUSTOM_WHISPER_PROVIDER {
        Some(build_custom_whisper_for_app(config).map_err(SttSetupError::InvalidCustomWhisper)?)
    } else {
        None
    };
    let stt_config = SttConfig {
        api_key,
        language: (config.stt_language != "multi").then(|| config.stt_language.clone()),
        smart_format: true,
        sample_rate: 16000,
        resource_id: (config.stt_provider == super::volcengine::VOLCENGINE_DOUBAO_PROVIDER)
            .then(|| config.stt_volcengine_resource_id.clone()),
        operation_id,
        managed_audio: super::capabilities::managed_audio_encoding_config(config, now_unix_seconds),
        provider_region: (config.stt_provider
            == super::aliyun_qwen3_asr::ALIYUN_QWEN3_ASR_PROVIDER)
            .then(|| config.stt_aliyun_qwen_region.clone()),
    };
    Ok(SttSetup {
        stt_config,
        custom_whisper,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Secrets {
        cloud: &'static str,
        vault: Result<&'static str, &'static str>,
    }

    impl SttSecrets for Secrets {
        fn cloud_session_token(&self) -> String {
            self.cloud.to_string()
        }
        fn stt_secret(&self, _config: &AppConfig) -> anyhow::Result<String> {
            self.vault
                .map(str::to_string)
                .map_err(|error| anyhow::anyhow!(error))
        }
    }

    const SECRETS: Secrets = Secrets {
        cloud: "session-token",
        vault: Ok("vault-key"),
    };

    fn config(provider: &str) -> AppConfig {
        AppConfig {
            stt_provider: provider.to_string(),
            ..Default::default()
        }
    }

    fn key(config: &AppConfig, secrets: &Secrets) -> Result<String, String> {
        resolve_stt_setup(config, secrets, None, 0)
            .map(|setup| setup.stt_config.api_key)
            .map_err(|error| format!("{error:?}"))
    }

    #[test]
    fn each_provider_takes_its_key_from_the_right_place() {
        let mut local = AppConfig::new_ultra_local_profile();
        local.stt_provider = CUSTOM_WHISPER_PROVIDER.to_string();
        assert_eq!(key(&local, &SECRETS), Ok(String::new()));
        assert_eq!(key(&config("cloud"), &SECRETS), Ok("session-token".into()));
        assert_eq!(key(&config("glm-asr"), &SECRETS), Ok("vault-key".into()));
    }

    #[test]
    fn a_missing_or_unreadable_key_stops_before_recording() {
        let empty_vault = Secrets {
            cloud: "",
            vault: Ok(""),
        };
        assert_eq!(
            key(&config("glm-asr"), &empty_vault),
            Err("MissingApiKey".into())
        );
        // Cloud sign-in is checked by the cloud service itself.
        assert_eq!(key(&config("cloud"), &empty_vault), Ok(String::new()));

        let locked_vault = Secrets {
            cloud: "",
            vault: Err("locked"),
        };
        assert!(key(&config("glm-asr"), &locked_vault)
            .unwrap_err()
            .starts_with("SecretUnavailable"));
    }

    #[test]
    fn provider_specific_settings_follow_the_provider() {
        let mut volcengine = config(super::super::volcengine::VOLCENGINE_DOUBAO_PROVIDER);
        volcengine.stt_volcengine_resource_id = "volc.bigasr".into();
        volcengine.stt_language = "ko".into();
        let setup = resolve_stt_setup(&volcengine, &SECRETS, Some("op-1".into()), 0).unwrap();
        assert_eq!(setup.stt_config.resource_id.as_deref(), Some("volc.bigasr"));
        assert_eq!(setup.stt_config.language.as_deref(), Some("ko"));
        assert_eq!(setup.stt_config.operation_id.as_deref(), Some("op-1"));
        assert_eq!(setup.stt_config.provider_region, None);
        assert!(setup.custom_whisper.is_none());

        let mut aliyun = config(super::super::aliyun_qwen3_asr::ALIYUN_QWEN3_ASR_PROVIDER);
        aliyun.stt_aliyun_qwen_region = "intl".into();
        aliyun.stt_language = "multi".into();
        let setup = resolve_stt_setup(&aliyun, &SECRETS, None, 0).unwrap();
        assert_eq!(setup.stt_config.provider_region.as_deref(), Some("intl"));
        assert_eq!(setup.stt_config.language, None);
        assert_eq!(setup.stt_config.resource_id, None);
    }

    #[test]
    fn custom_whisper_carries_its_endpoint_or_explains_the_problem() {
        let mut local = AppConfig::new_ultra_local_profile();
        local.stt_provider = CUSTOM_WHISPER_PROVIDER.to_string();
        let setup = resolve_stt_setup(&local, &SECRETS, None, 0).unwrap();
        assert!(setup.custom_whisper.is_some());

        let mut broken = local.clone();
        broken.stt_custom_model = "other-model".into();
        assert!(matches!(
            resolve_stt_setup(&broken, &SECRETS, None, 0),
            Err(SttSetupError::InvalidCustomWhisper(_))
        ));
    }
}
