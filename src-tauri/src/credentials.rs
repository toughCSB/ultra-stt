use anyhow::{anyhow, Context, Result};
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};

use crate::storage::AppConfig;

const SERVICE_NAME: &str = "OpenTypeless";
const API_KEY_ACCOUNT_SUFFIX: &str = "api_key";
const STORED_CREDENTIAL_VERSION: u8 = 1;
const CLOUD_SESSION_NAMESPACE: &str = "session";
const CLOUD_SESSION_PROVIDER: &str = "cloud";

pub trait CredentialVault {
    fn set_secret(&self, namespace: &str, provider: &str, secret: &str) -> Result<()>;
}

pub trait CredentialSecretReader {
    fn get_secret(&self, namespace: &str, provider: &str) -> Result<Option<String>>;

    fn get_secret_updated_at(&self, _namespace: &str, _provider: &str) -> Result<Option<String>> {
        Ok(None)
    }
}

pub trait CredentialSecretRemover {
    fn remove_secret(&self, namespace: &str, provider: &str) -> Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialRef {
    pub namespace: String,
    pub provider: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CredentialMigrationReport {
    pub migrated: Vec<CredentialRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCredential {
    pub value: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCredentialPayload {
    version: u8,
    secret_kind: String,
    value: String,
    updated_at: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemCredentialVault;

impl SystemCredentialVault {
    fn account(namespace: &str, provider: &str) -> String {
        format!(
            "{}.{}.{}",
            namespace.trim(),
            provider.trim(),
            API_KEY_ACCOUNT_SUFFIX
        )
    }

    pub fn has_secret(&self, namespace: &str, provider: &str) -> Result<bool> {
        Ok(self.get_secret(namespace, provider)?.is_some())
    }

    pub fn remove_secret(&self, namespace: &str, provider: &str) -> Result<()> {
        delete_system_secret(&Self::account(namespace, provider))
    }
}

impl CredentialVault for SystemCredentialVault {
    fn set_secret(&self, namespace: &str, provider: &str, secret: &str) -> Result<()> {
        let stored = encode_stored_credential(secret, &current_credential_timestamp())
            .map_err(|e| anyhow!("encode credential payload for {namespace}.{provider}: {e}"))?;
        write_system_secret(&Self::account(namespace, provider), &stored)
    }
}

impl CredentialSecretReader for SystemCredentialVault {
    fn get_secret(&self, namespace: &str, provider: &str) -> Result<Option<String>> {
        read_system_secret(&Self::account(namespace, provider))?
            .map(|stored| decode_stored_credential(&stored).map(|credential| credential.value))
            .transpose()
    }

    fn get_secret_updated_at(&self, namespace: &str, provider: &str) -> Result<Option<String>> {
        read_system_secret(&Self::account(namespace, provider))?
            .map(|stored| decode_stored_credential(&stored).map(|credential| credential.updated_at))
            .transpose()
            .map(Option::flatten)
    }
}

impl CredentialSecretRemover for SystemCredentialVault {
    fn remove_secret(&self, namespace: &str, provider: &str) -> Result<()> {
        Self::remove_secret(self, namespace, provider)
    }
}

pub fn load_cloud_session_token<V: CredentialSecretReader>(vault: &V) -> Result<Option<String>> {
    Ok(vault
        .get_secret(CLOUD_SESSION_NAMESPACE, CLOUD_SESSION_PROVIDER)?
        .filter(|token| !token.trim().is_empty()))
}

pub fn store_cloud_session_token<V: CredentialVault + CredentialSecretReader>(
    vault: &V,
    token: &str,
) -> Result<()> {
    if token.trim().is_empty() {
        return Err(anyhow!("cloud session token is empty"));
    }
    vault.set_secret(CLOUD_SESSION_NAMESPACE, CLOUD_SESSION_PROVIDER, token)?;
    if load_cloud_session_token(vault)?.as_deref() != Some(token) {
        return Err(anyhow!("cloud session vault verification failed"));
    }
    Ok(())
}

pub fn remove_cloud_session_token<V: CredentialSecretRemover>(vault: &V) -> Result<()> {
    vault.remove_secret(CLOUD_SESSION_NAMESPACE, CLOUD_SESSION_PROVIDER)
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn system_entry(account: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE_NAME, account).context("open system credential vault")
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn write_system_secret(account: &str, secret: &str) -> Result<()> {
    with_system_vault(|| {
        system_entry(account)?
            .set_password(secret)
            .with_context(|| format!("write system credential vault {account}"))
    })
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn write_system_secret(_account: &str, _secret: &str) -> Result<()> {
    Err(anyhow!(
        "system credential vault is not supported on this platform"
    ))
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn read_system_secret(account: &str) -> Result<Option<String>> {
    with_system_vault(|| match system_entry(account)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(anyhow!("read system credential vault {account}: {error}")),
    })
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn read_system_secret(_account: &str) -> Result<Option<String>> {
    Err(anyhow!(
        "system credential vault is not supported on this platform"
    ))
}

#[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
fn delete_system_secret(account: &str) -> Result<()> {
    with_system_vault(|| match system_entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(anyhow!("delete system credential vault {account}: {error}")),
    })
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn delete_system_secret(_account: &str) -> Result<()> {
    Err(anyhow!(
        "system credential vault is not supported on this platform"
    ))
}

#[cfg(target_os = "macos")]
fn with_system_vault<Value>(operation: impl FnOnce() -> Result<Value>) -> Result<Value> {
    #[link(name = "Security", kind = "framework")]
    extern "C" {
        fn SecKeychainGetUserInteractionAllowed(state: *mut u8) -> i32;
        fn SecKeychainSetUserInteractionAllowed(state: u8) -> i32;
    }

    static KEYCHAIN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _lock = KEYCHAIN_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut previous = 0;
    let status = unsafe { SecKeychainGetUserInteractionAllowed(&mut previous) };
    if status != 0 {
        return Err(anyhow!("read Keychain interaction state: {status}"));
    }
    let status = unsafe { SecKeychainSetUserInteractionAllowed(0) };
    if status != 0 {
        return Err(anyhow!("disable Keychain interaction for vault operation: {status}"));
    }

    struct RestoreInteraction(u8);
    impl Drop for RestoreInteraction {
        fn drop(&mut self) {
            let status = unsafe { SecKeychainSetUserInteractionAllowed(self.0) };
            if status != 0 {
                tracing::warn!(status, "Could not restore Keychain interaction state");
            }
        }
    }
    let _restore = RestoreInteraction(previous);
    operation()
}

#[cfg(not(target_os = "macos"))]
fn with_system_vault<Value>(operation: impl FnOnce() -> Result<Value>) -> Result<Value> {
    operation()
}

fn current_credential_timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn encode_stored_credential(secret: &str, updated_at: &str) -> Result<String> {
    serde_json::to_string(&StoredCredentialPayload {
        version: STORED_CREDENTIAL_VERSION,
        secret_kind: "apiKey".to_string(),
        value: secret.to_string(),
        updated_at: updated_at.to_string(),
    })
    .context("serialize credential payload")
}

fn decode_stored_credential(stored: &str) -> Result<StoredCredential> {
    match serde_json::from_str::<StoredCredentialPayload>(stored) {
        Ok(payload) if payload.version == STORED_CREDENTIAL_VERSION => Ok(StoredCredential {
            value: payload.value,
            updated_at: Some(payload.updated_at),
        }),
        _ => Ok(StoredCredential {
            value: stored.to_string(),
            updated_at: None,
        }),
    }
}

pub fn migrate_legacy_config_secrets<V: CredentialVault + CredentialSecretReader>(
    config: &mut AppConfig,
    vault: &V,
) -> Result<CredentialMigrationReport> {
    let mut pending = Vec::new();

    if !config.stt_api_key.trim().is_empty() {
        pending.push((
            "stt".to_string(),
            config.stt_provider.clone(),
            config.stt_api_key.clone(),
        ));
    }
    if !config.stt_custom_api_key.trim().is_empty() {
        pending.push((
            "stt".to_string(),
            "custom-whisper".to_string(),
            config.stt_custom_api_key.clone(),
        ));
    }
    if !config.llm_api_key.trim().is_empty() {
        pending.push((
            "llm".to_string(),
            config.llm_provider.clone(),
            config.llm_api_key.clone(),
        ));
    }

    for (namespace, provider, secret) in &pending {
        vault.set_secret(namespace, provider, secret)?;
        let verified = vault.get_secret(namespace, provider)?;
        if verified.as_deref() != Some(secret.as_str()) {
            return Err(anyhow!(
                "credential vault verification failed for {namespace}.{provider}"
            ));
        }
    }

    config.stt_api_key.clear();
    config.stt_custom_api_key.clear();
    config.llm_api_key.clear();

    Ok(CredentialMigrationReport {
        migrated: pending
            .into_iter()
            .map(|(namespace, provider, _)| CredentialRef {
                namespace,
                provider,
            })
            .collect(),
    })
}

pub fn resolve_config_secret<V: CredentialSecretReader>(
    legacy_secret: &str,
    namespace: &str,
    provider: &str,
    vault: &V,
) -> Result<String> {
    if !legacy_secret.trim().is_empty() {
        return Ok(legacy_secret.to_string());
    }
    // A keychain that refuses to answer, which macOS does when the app has no
    // unlocked session, must surface as a missing key rather than a hard error,
    // otherwise dictation fails with an opaque message.
    match vault.get_secret(namespace, provider) {
        Ok(secret) => Ok(secret.unwrap_or_default()),
        Err(error) => {
            tracing::warn!(
                "Could not read the {namespace}.{provider} credential: {error}"
            );
            Ok(String::new())
        }
    }
}

pub fn stt_credential_provider(config: &AppConfig) -> &str {
    if config.stt_provider == crate::stt::config::CUSTOM_WHISPER_PROVIDER {
        crate::stt::config::CUSTOM_WHISPER_PROVIDER
    } else {
        &config.stt_provider
    }
}

pub fn resolve_stt_config_secret<V: CredentialSecretReader>(
    config: &AppConfig,
    vault: &V,
) -> Result<String> {
    let provider = stt_credential_provider(config);
    if provider == crate::stt::config::CUSTOM_WHISPER_PROVIDER {
        // A custom endpoint key can land in either field depending on whether the
        // user typed it into the custom box or the shared one, so accept both
        // before falling back to the vault.
        if !config.stt_custom_api_key.trim().is_empty() {
            return Ok(config.stt_custom_api_key.clone());
        }
        if !config.stt_api_key.trim().is_empty() {
            return Ok(config.stt_api_key.clone());
        }
    } else if !config.stt_api_key.trim().is_empty() {
        return Ok(config.stt_api_key.clone());
    }

    Ok(vault.get_secret("stt", provider)?.unwrap_or_default())
}

pub fn resolve_llm_config_secret<V: CredentialSecretReader>(
    config: &AppConfig,
    vault: &V,
) -> Result<String> {
    resolve_config_secret(&config.llm_api_key, "llm", &config.llm_provider, vault)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryVault {
        records: Mutex<Vec<(String, String, String)>>,
        fail_provider: Option<String>,
    }

    impl MemoryVault {
        fn with_failure(provider: &str) -> Self {
            Self {
                records: Mutex::new(Vec::new()),
                fail_provider: Some(provider.to_string()),
            }
        }

        fn records(&self) -> Vec<(String, String, String)> {
            self.records.lock().unwrap().clone()
        }
    }

    impl CredentialVault for MemoryVault {
        fn set_secret(&self, namespace: &str, provider: &str, secret: &str) -> Result<()> {
            if self.fail_provider.as_deref() == Some(provider) {
                return Err(anyhow!("vault write failed"));
            }
            self.records.lock().unwrap().push((
                namespace.to_string(),
                provider.to_string(),
                secret.to_string(),
            ));
            Ok(())
        }
    }

    impl CredentialSecretReader for MemoryVault {
        fn get_secret(&self, namespace: &str, provider: &str) -> Result<Option<String>> {
            Ok(self
                .records
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|(record_namespace, record_provider, _)| {
                    record_namespace == namespace && record_provider == provider
                })
                .map(|(_, _, secret)| secret.clone()))
        }
    }

    #[test]
    fn migrates_plaintext_api_keys_and_clears_config_after_success() {
        let mut config = AppConfig {
            stt_provider: "deepgram".to_string(),
            stt_api_key: "stt-secret".to_string(),
            stt_custom_api_key: "custom-stt-secret".to_string(),
            llm_provider: "openai".to_string(),
            llm_api_key: "llm-secret".to_string(),
            ..AppConfig::default()
        };
        let vault = MemoryVault::default();

        let report = migrate_legacy_config_secrets(&mut config, &vault).unwrap();

        assert_eq!(
            vault.records(),
            vec![
                (
                    "stt".to_string(),
                    "deepgram".to_string(),
                    "stt-secret".to_string()
                ),
                (
                    "stt".to_string(),
                    "custom-whisper".to_string(),
                    "custom-stt-secret".to_string()
                ),
                (
                    "llm".to_string(),
                    "openai".to_string(),
                    "llm-secret".to_string()
                ),
            ]
        );
        assert_eq!(
            report.migrated,
            vec![
                CredentialRef {
                    namespace: "stt".to_string(),
                    provider: "deepgram".to_string(),
                },
                CredentialRef {
                    namespace: "stt".to_string(),
                    provider: "custom-whisper".to_string(),
                },
                CredentialRef {
                    namespace: "llm".to_string(),
                    provider: "openai".to_string(),
                },
            ]
        );
        assert!(config.stt_api_key.is_empty());
        assert!(config.stt_custom_api_key.is_empty());
        assert!(config.llm_api_key.is_empty());
    }

    #[test]
    fn migrates_custom_provider_shared_key_once() {
        let mut config = AppConfig {
            stt_provider: "custom-whisper".to_string(),
            stt_api_key: "shared-custom-secret".to_string(),
            ..AppConfig::default()
        };
        let vault = MemoryVault::default();

        migrate_legacy_config_secrets(&mut config, &vault).unwrap();

        assert!(config.stt_api_key.is_empty());
        assert_eq!(
            resolve_stt_config_secret(&config, &vault).unwrap(),
            "shared-custom-secret"
        );
        assert!(migrate_legacy_config_secrets(&mut config, &vault)
            .unwrap()
            .migrated
            .is_empty());
        assert_eq!(vault.records().len(), 1);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn vault_operation_restores_interaction_after_success_error_and_panic() {
        #[link(name = "Security", kind = "framework")]
        extern "C" {
            fn SecKeychainGetUserInteractionAllowed(state: *mut u8) -> i32;
        }
        let read_state = || {
            let mut state = 0;
            assert_eq!(unsafe { SecKeychainGetUserInteractionAllowed(&mut state) }, 0);
            state
        };
        let previous = read_state();

        with_system_vault(|| {
            assert_eq!(read_state(), 0);
            Ok(())
        })
        .unwrap();
        assert_eq!(read_state(), previous);

        let failure: Result<()> = with_system_vault(|| Err(anyhow!("fixture failure")));
        assert!(failure.is_err());
        assert_eq!(read_state(), previous);

        assert!(std::panic::catch_unwind(|| {
            let _: Result<()> = with_system_vault(|| panic!("fixture panic"));
        })
        .is_err());
        assert_eq!(read_state(), previous);
        assert!(with_system_vault(|| Ok(())).is_ok());
    }

    #[test]
    fn keeps_plaintext_api_keys_when_any_vault_write_fails() {
        let mut config = AppConfig {
            stt_provider: "deepgram".to_string(),
            stt_api_key: "stt-secret".to_string(),
            stt_custom_api_key: "custom-stt-secret".to_string(),
            llm_provider: "openai".to_string(),
            llm_api_key: "llm-secret".to_string(),
            ..AppConfig::default()
        };
        let vault = MemoryVault::with_failure("openai");

        let result = migrate_legacy_config_secrets(&mut config, &vault);

        assert!(result.is_err());
        assert_eq!(config.stt_api_key, "stt-secret");
        assert_eq!(config.stt_custom_api_key, "custom-stt-secret");
        assert_eq!(config.llm_api_key, "llm-secret");
    }

    #[test]
    fn keeps_plaintext_api_keys_when_vault_verification_fails() {
        struct MismatchedReadVault;

        impl CredentialVault for MismatchedReadVault {
            fn set_secret(&self, _namespace: &str, _provider: &str, _secret: &str) -> Result<()> {
                Ok(())
            }
        }

        impl CredentialSecretReader for MismatchedReadVault {
            fn get_secret(&self, _namespace: &str, _provider: &str) -> Result<Option<String>> {
                Ok(Some("wrong-secret".to_string()))
            }
        }

        let mut config = AppConfig {
            stt_provider: "deepgram".to_string(),
            stt_api_key: "stt-secret".to_string(),
            ..AppConfig::default()
        };

        let result = migrate_legacy_config_secrets(&mut config, &MismatchedReadVault);

        assert!(result.is_err());
        assert_eq!(config.stt_api_key, "stt-secret");
    }

    #[test]
    fn resolves_missing_config_secret_from_vault() {
        struct ReadVault;

        impl CredentialSecretReader for ReadVault {
            fn get_secret(&self, namespace: &str, provider: &str) -> Result<Option<String>> {
                assert_eq!(namespace, "llm");
                assert_eq!(provider, "openai");
                Ok(Some("vault-secret".to_string()))
            }
        }

        let secret = resolve_config_secret("", "llm", "openai", &ReadVault).unwrap();

        assert_eq!(secret, "vault-secret");
    }

    #[test]
    fn resolves_in_memory_secret_before_vault() {
        struct PanicVault;

        impl CredentialSecretReader for PanicVault {
            fn get_secret(&self, _namespace: &str, _provider: &str) -> Result<Option<String>> {
                panic!("vault should not be read when legacy secret is present");
            }
        }

        let secret = resolve_config_secret("typed-secret", "llm", "openai", &PanicVault).unwrap();

        assert_eq!(secret, "typed-secret");
    }

    #[test]
    fn stored_credential_payload_round_trips_secret_and_metadata() {
        let stored = encode_stored_credential("vault-secret", "2026-07-06T00:00:00Z").unwrap();

        assert_ne!(stored, "vault-secret");

        let decoded = decode_stored_credential(&stored).unwrap();

        assert_eq!(decoded.value, "vault-secret");
        assert_eq!(decoded.updated_at.as_deref(), Some("2026-07-06T00:00:00Z"));
    }

    #[test]
    fn stored_credential_decoder_preserves_legacy_plaintext_secret() {
        let decoded = decode_stored_credential("legacy-secret").unwrap();

        assert_eq!(decoded.value, "legacy-secret");
        assert_eq!(decoded.updated_at, None);
    }

    #[test]
    fn resolves_regular_stt_secret_from_active_provider() {
        let config = AppConfig {
            stt_provider: "deepgram".to_string(),
            ..AppConfig::default()
        };
        let vault = MemoryVault::default();
        vault
            .set_secret("stt", "deepgram", "deepgram-secret")
            .unwrap();

        let secret = resolve_stt_config_secret(&config, &vault).unwrap();

        assert_eq!(secret, "deepgram-secret");
    }

    #[test]
    fn resolves_custom_stt_secret_from_custom_whisper_provider() {
        let config = AppConfig {
            stt_provider: crate::stt::config::CUSTOM_WHISPER_PROVIDER.to_string(),
            ..AppConfig::default()
        };
        let vault = MemoryVault::default();
        vault
            .set_secret(
                "stt",
                crate::stt::config::CUSTOM_WHISPER_PROVIDER,
                "custom-secret",
            )
            .unwrap();

        let secret = resolve_stt_config_secret(&config, &vault).unwrap();

        assert_eq!(secret, "custom-secret");
    }

    #[test]
    fn resolves_llm_secret_from_active_provider() {
        let config = AppConfig {
            llm_provider: "openai".to_string(),
            ..AppConfig::default()
        };
        let vault = MemoryVault::default();
        vault.set_secret("llm", "openai", "llm-secret").unwrap();

        let secret = resolve_llm_config_secret(&config, &vault).unwrap();

        assert_eq!(secret, "llm-secret");
    }

    #[test]
    fn custom_whisper_uses_the_shared_key_field_when_the_custom_one_is_empty() {
        // The key can land in either field depending on which box the user typed
        // it into, and on macOS the custom field is often the one left empty.
        let mut config = AppConfig::new_install_default();
        config.stt_provider = crate::stt::config::CUSTOM_WHISPER_PROVIDER.to_string();
        config.stt_custom_api_key = String::new();
        config.stt_api_key = "shared-token".to_string();

        let vault = MemoryVault::default();
        let resolved = resolve_stt_config_secret(&config, &vault).expect("resolution should not fail");
        assert_eq!(resolved, "shared-token");
    }

    #[test]
    fn custom_whisper_prefers_its_own_key_field() {
        let mut config = AppConfig::new_install_default();
        config.stt_provider = crate::stt::config::CUSTOM_WHISPER_PROVIDER.to_string();
        config.stt_custom_api_key = "custom-token".to_string();
        config.stt_api_key = "shared-token".to_string();

        let vault = MemoryVault::default();
        let resolved = resolve_stt_config_secret(&config, &vault).expect("resolution should not fail");
        assert_eq!(resolved, "custom-token");
    }

    #[test]
    fn an_unreadable_vault_yields_no_key_instead_of_an_error() {
        // macOS refuses the keychain when the app has no unlocked session. That
        // must read as a missing key, not as a hard recognition failure.
        let mut config = AppConfig::new_install_default();
        config.stt_provider = "some-cloud-provider".to_string();
        config.stt_api_key = String::new();

        let vault = MemoryVault::with_failure("some-cloud-provider");
        let resolved = resolve_stt_config_secret(&config, &vault)
            .expect("a vault read failure must not propagate");
        assert!(resolved.is_empty());
    }
}
