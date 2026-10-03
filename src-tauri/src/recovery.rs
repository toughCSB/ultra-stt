//! Short-lived failed-output recovery, independent of persistent history.
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

use crate::output::{InsertResult, InsertStatus};

const MAX_ENTRIES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryReason {
    TargetChanged,
    PartialInsert,
    OutputFailed,
    ClipboardFallback,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryEntry {
    pub id: String,
    pub session_id: u64,
    pub created_at: String,
    pub text: String,
    pub app_name: String,
    pub reason: RecoveryReason,
    pub chars_inserted: usize,
}

#[derive(Default)]
pub struct RecoveryStore(Mutex<VecDeque<RecoveryEntry>>);

impl RecoveryStore {
    pub fn entries(&self) -> Vec<RecoveryEntry> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    fn retain(
        &self,
        session_id: u64,
        text: &str,
        app_name: &str,
        reason: RecoveryReason,
        chars_inserted: usize,
    ) -> bool {
        if text.trim().is_empty() {
            return false;
        }
        let mut entries = self.0.lock().unwrap_or_else(|e| e.into_inner());
        // A voice executor can report the same failure again after its clipboard fallback.
        if entries
            .iter()
            .any(|entry| entry.session_id == session_id && entry.text == text)
        {
            return false;
        }
        entries.push_front(RecoveryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            created_at: chrono::Utc::now().to_rfc3339(),
            text: text.to_string(),
            app_name: app_name.to_string(),
            reason,
            chars_inserted,
        });
        entries.truncate(MAX_ENTRIES);
        true
    }

    fn dismiss(&self, id: &str) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|entry| entry.id != id);
    }

    fn text(&self, id: &str) -> Result<String, String> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.text.clone())
            .ok_or_else(|| "Recovery entry is no longer available".to_string())
    }
}

pub fn reason_for_result(result: &InsertResult, intentional_copy: bool) -> Option<RecoveryReason> {
    if result.warning_code.as_deref() == Some("output_target_changed") {
        return Some(RecoveryReason::TargetChanged);
    }
    match result.status {
        InsertStatus::PartiallyInserted => Some(RecoveryReason::PartialInsert),
        InsertStatus::Failed => Some(RecoveryReason::OutputFailed),
        InsertStatus::CopiedFallback if !intentional_copy || result.warning_code.is_some() => {
            Some(RecoveryReason::ClipboardFallback)
        }
        _ => None,
    }
}

pub fn reason_for_voice_fallback(
    reason: Option<crate::voice_intent::executor::VoiceExecutionFallbackReason>,
    intentional_copy: bool,
    already_copied: bool,
) -> Option<RecoveryReason> {
    use crate::voice_intent::executor::VoiceExecutionFallbackReason as Reason;
    match reason {
        Some(Reason::TargetChanged | Reason::SelectionLost | Reason::FocusRestoreFailed) => {
            Some(RecoveryReason::TargetChanged)
        }
        Some(Reason::OutputFailed) if !intentional_copy || !already_copied => {
            Some(RecoveryReason::OutputFailed)
        }
        _ => None,
    }
}

pub fn retain(
    app: &tauri::AppHandle,
    session_id: u64,
    text: &str,
    app_name: &str,
    reason: RecoveryReason,
    chars_inserted: usize,
) {
    let store = app.state::<RecoveryStore>();
    if store.retain(session_id, text, app_name, reason, chars_inserted) {
        let _ = app.emit("pipeline:recovery", store.entries());
    }
}

#[tauri::command]
pub fn get_recovery_entries(state: tauri::State<'_, RecoveryStore>) -> Vec<RecoveryEntry> {
    state.entries()
}

#[tauri::command]
pub fn dismiss_recovery_entry(
    app: tauri::AppHandle,
    state: tauri::State<'_, RecoveryStore>,
    id: String,
) -> Vec<RecoveryEntry> {
    state.dismiss(&id);
    let entries = state.entries();
    let _ = app.emit("pipeline:recovery", &entries);
    entries
}

#[tauri::command]
pub async fn copy_recovery_entry(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let text = app.state::<RecoveryStore>().text(&id)?;
    // Explicit copy only: never focus, restore a target, or simulate a paste.
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(text)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::InsertionStrategy;

    #[test]
    fn bounded_queue_preserves_full_text_and_deduplicates() {
        let store = RecoveryStore::default();
        let text = "한글\nfull result ".repeat(1000);
        assert!(store.retain(1, &text, "Editor", RecoveryReason::PartialInsert, 7));
        assert!(!store.retain(1, &text, "Editor", RecoveryReason::OutputFailed, 0));
        assert_eq!(store.entries()[0].text, text);
        assert_eq!(store.entries()[0].chars_inserted, 7);
        for session in 2..=25 {
            store.retain(
                session,
                "result",
                "Editor",
                RecoveryReason::TargetChanged,
                0,
            );
        }
        assert_eq!(store.entries().len(), MAX_ENTRIES);
        assert_eq!(store.entries()[0].session_id, 25);
        assert!(!store.retain(26, "  ", "Editor", RecoveryReason::OutputFailed, 0));
    }

    #[test]
    fn dismiss_removes_only_requested_entry_and_copy_does_not_consume() {
        let store = RecoveryStore::default();
        store.retain(1, "first", "Editor", RecoveryReason::OutputFailed, 0);
        store.retain(2, "second", "Editor", RecoveryReason::OutputFailed, 0);
        let entry = store.entries()[0].clone();
        assert_eq!(store.text(&entry.id).unwrap(), "second");
        assert_eq!(store.entries().len(), 2);
        store.dismiss(&entry.id);
        assert!(store.text(&entry.id).is_err());
        assert_eq!(store.entries()[0].text, "first");
    }

    #[test]
    fn deliberate_voice_copy_is_not_a_failed_insert() {
        use crate::voice_intent::executor::VoiceExecutionFallbackReason as Reason;
        assert_eq!(
            reason_for_voice_fallback(Some(Reason::OutputFailed), true, true),
            None
        );
        assert_eq!(
            reason_for_voice_fallback(Some(Reason::OutputFailed), true, false),
            Some(RecoveryReason::OutputFailed)
        );
        assert_eq!(
            reason_for_voice_fallback(Some(Reason::OutputFailed), false, true),
            Some(RecoveryReason::OutputFailed)
        );
        assert_eq!(
            reason_for_voice_fallback(Some(Reason::TargetChanged), false, true),
            Some(RecoveryReason::TargetChanged)
        );
        assert_eq!(
            reason_for_voice_fallback(Some(Reason::FeatureDisabled), false, false),
            None
        );
    }

    #[test]
    fn deliberate_copy_and_success_do_not_create_recovery() {
        let copied = InsertResult::copied_fallback(InsertionStrategy::ClipboardCopyOnly, 10);
        assert_eq!(reason_for_result(&copied, true), None);
        assert_eq!(
            reason_for_result(&copied, false),
            Some(RecoveryReason::ClipboardFallback)
        );
        assert_eq!(
            reason_for_result(
                &InsertResult::inserted(InsertionStrategy::Keyboard, 10),
                false
            ),
            None
        );
        assert_eq!(
            reason_for_result(
                &InsertResult::partially_inserted(InsertionStrategy::Keyboard, 3),
                false
            ),
            Some(RecoveryReason::PartialInsert)
        );
        assert_eq!(
            reason_for_result(&InsertResult::failed(InsertionStrategy::Keyboard), false),
            Some(RecoveryReason::OutputFailed)
        );
    }
}
