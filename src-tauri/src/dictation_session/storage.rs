use async_trait::async_trait;

use crate::llm::CorrectionRule;
use crate::recording_archive::RecordingJournal;
use crate::storage::{AppConfig, HistoryEntry, HistoryRetentionPolicy};

/// Settings and secrets a dictation session reads.
#[async_trait]
pub(crate) trait SessionSettings: Send + Sync {
    /// Current settings; the defaults when they cannot be read.
    async fn load_config(&self) -> AppConfig;
    /// Signed-in cloud session token; empty when signed out.
    fn cloud_session_token(&self) -> String;
    fn stt_secret(&self, config: &AppConfig) -> anyhow::Result<String>;
    fn llm_secret(&self, config: &AppConfig) -> anyhow::Result<String>;
}

/// Records a dictation session keeps: the dictionary it applies, the recovery audio it
/// writes while recording, and the history entry it leaves at the end.
#[async_trait]
pub(crate) trait SessionStores: Send + Sync {
    async fn dictionary_words(&self) -> Vec<String>;
    /// Enabled correction rules, in the order they are applied.
    async fn correction_rules(&self) -> Vec<CorrectionRule>;
    fn begin_recording(&self) -> std::io::Result<RecordingJournal>;
    async fn add_history(
        &self,
        entry: HistoryEntry,
        policy: &HistoryRetentionPolicy,
    ) -> anyhow::Result<()>;
}

/// Converts stored correction rules into the form the AI polish request carries.
pub(crate) fn correction_rules_for_polish(
    rules: Vec<crate::storage::CorrectionRule>,
) -> Vec<CorrectionRule> {
    rules
        .into_iter()
        .map(|rule| CorrectionRule {
            id: rule.id,
            pattern: rule.pattern,
            replacement: rule.replacement,
            enabled: rule.enabled,
        })
        .collect()
}
