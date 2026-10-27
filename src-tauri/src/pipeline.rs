use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};
use tokio::sync::{mpsc, Notify};

use crate::app_detector;
use crate::app_detector::types::{RecordingContext, TargetAppGuard};
use crate::audio::{AudioCaptureHandle, AudioConfig};
use crate::credentials::{
    resolve_llm_config_secret, resolve_stt_config_secret, SystemCredentialVault,
};
use crate::dictation_session::{Phases, SessionClock, SessionNotifier, StateCell};
use crate::llm::{self, LlmConfig, PolishRequest};
use crate::output;
use crate::storage;
use crate::stt::{self, SttConfig, TranscriptEvent};
use crate::SessionTokenStore;

// ─── Timing constants ───

/// On macOS, verify whether the process has been granted Accessibility (Assistive Access)
/// permission. enigo uses CGEventPost under the hood, which requires this permission;
/// without it all synthesised key events are silently dropped by the OS.
/// Returns true on all non-macOS platforms (no permission needed).
pub fn is_accessibility_trusted() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> u8;
        }
        unsafe { AXIsProcessTrusted() != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// On macOS, request Accessibility permission and open the system Privacy pane.
/// Returns true if permission is already granted or on non-macOS platforms.
pub fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        if is_accessibility_trusted() {
            return true;
        }

        let trusted_after_prompt = request_accessibility_permission_prompt();

        if let Err(e) = std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
        {
            tracing::warn!("Failed to open macOS Accessibility settings: {}", e);
        }

        trusted_after_prompt
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(target_os = "macos")]
fn request_accessibility_permission_prompt() -> bool {
    use std::ffi::c_void;
    use std::ptr;

    type Boolean = u8;
    type CFDictionaryRef = *const c_void;
    type CFStringRef = *const c_void;
    type CFBooleanRef = *const c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> Boolean;
        static kAXTrustedCheckOptionPrompt: CFStringRef;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFBooleanTrue: CFBooleanRef;
        fn CFDictionaryCreate(
            allocator: *const c_void,
            keys: *const *const c_void,
            values: *const *const c_void,
            num_values: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CFDictionaryRef;
        fn CFRelease(cf: *const c_void);
    }

    unsafe {
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            ptr::null(),
            ptr::null(),
        );

        if options.is_null() {
            return false;
        }

        let trusted = AXIsProcessTrustedWithOptions(options) != 0;
        CFRelease(options);
        trusted
    }
}

/// Delay before capturing selected text to ensure hotkey modifiers are released.
const SELECTED_TEXT_CAPTURE_DELAY_MS: u64 = 60;
/// Interval for polling audio volume during recording.
const VOLUME_POLL_INTERVAL_MS: u64 = 50;

fn generate_cloud_operation_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let mixed = now ^ (counter << 64);

    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        (mixed >> 96) as u32,
        (mixed >> 80) as u16,
        (mixed >> 64) as u16,
        (mixed >> 48) as u16,
        mixed & 0x0000_ffff_ffff_ffff_ffffu128
    )
}

pub use crate::dictation_session::PipelineState;

#[derive(Clone)]
struct SttTaskControl {
    id: u64,
    done: Arc<Notify>,
    abort: Arc<Notify>,
    remote_polish: Arc<Mutex<Option<stt::RemotePolishOptions>>>,
    raw_transcript: Arc<Mutex<Option<String>>>,
    journal: Arc<Mutex<crate::recording_archive::RecordingJournal>>,
}

fn remote_polish_options(config: &storage::AppConfig) -> Option<stt::RemotePolishOptions> {
    (config.stt_provider == stt::config::CUSTOM_WHISPER_PROVIDER
        && config.stt_custom_preset == stt::config::ULTRA_REMOTE_STT_PRESET)
        .then(|| stt::RemotePolishOptions {
            style: config.polish_style.clone(),
            enabled: config.polish_enabled,
        })
}

fn no_speech_user_error() -> crate::error::UserError {
    crate::error::UserError {
        code: "stt_no_speech_detected".to_string(),
        details: None,
        retry_count: 0,
    }
}

fn llm_polish_user_error(error: &crate::error::AppError) -> crate::error::UserError {
    if matches!(error, crate::error::AppError::LlmQuota(_)) {
        return error.to_user_error();
    }

    crate::error::UserError {
        code: "llm_failed".to_string(),
        details: Some(error.to_string()),
        retry_count: 0,
    }
}

fn output_user_error(error: &anyhow::Error) -> crate::error::UserError {
    let details = error.to_string();
    let is_accessibility_error = details.contains("ACCESSIBILITY_REQUIRED");

    crate::error::UserError {
        code: if is_accessibility_error {
            "accessibility_required"
        } else {
            "output_failed"
        }
        .to_string(),
        details: if is_accessibility_error {
            None
        } else {
            Some(details)
        },
        retry_count: 0,
    }
}

fn accessibility_required_user_error() -> crate::error::UserError {
    crate::error::UserError {
        code: "accessibility_required".to_string(),
        details: None,
        retry_count: 0,
    }
}

fn strategy_uses_automated_input(strategy: output::InsertionStrategy) -> bool {
    matches!(
        strategy,
        output::InsertionStrategy::Auto
            | output::InsertionStrategy::Keyboard
            | output::InsertionStrategy::ClipboardPaste
            | output::InsertionStrategy::WindowsSendInput
    )
}

fn effective_strategy_for_accessibility(
    strategy: output::InsertionStrategy,
    accessibility_trusted: bool,
) -> (output::InsertionStrategy, Option<crate::error::UserError>) {
    if !accessibility_trusted && strategy_uses_automated_input(strategy) {
        return (
            output::InsertionStrategy::ClipboardCopyOnly,
            Some(accessibility_required_user_error()),
        );
    }

    (strategy, None)
}

fn streaming_insert_user_error(details: Option<String>) -> crate::error::UserError {
    crate::error::UserError {
        code: "output_streaming_partial".to_string(),
        details,
        retry_count: 0,
    }
}

fn selected_text_has_content(selected_text: Option<&str>) -> bool {
    selected_text.is_some_and(|text| !text.trim().is_empty())
}

fn remote_output_is_final(
    config: &storage::AppConfig,
    mode: crate::voice_intent::VoiceMode,
    kind: crate::voice_intent::VoiceIntentKind,
    selected: Option<&str>,
) -> bool {
    config.stt_provider == crate::stt::config::CUSTOM_WHISPER_PROVIDER
        && config.stt_custom_preset == crate::stt::config::ULTRA_REMOTE_STT_PRESET
        && mode == crate::voice_intent::VoiceMode::Dictate
        && kind == crate::voice_intent::VoiceIntentKind::DictateInsert
        && !selected_text_has_content(selected)
        && !config.translate_enabled
}

#[cfg(test)]
mod remote_command_regression {
    use super::*;
    use crate::voice_intent::{VoiceIntentKind, VoiceMode};
    #[test]
    fn remote_dictation_keeps_its_fast_path_but_commands_reach_the_llm() {
        let mut config = storage::AppConfig::default();
        config.stt_provider = crate::stt::config::CUSTOM_WHISPER_PROVIDER.into();
        config.stt_custom_preset = crate::stt::config::ULTRA_REMOTE_STT_PRESET.into();
        config.translate_enabled = false;
        assert!(remote_output_is_final(
            &config,
            VoiceMode::Dictate,
            VoiceIntentKind::DictateInsert,
            None
        ));
        for (mode, kind, selected) in [
            (
                VoiceMode::Dictate,
                VoiceIntentKind::RewriteSelection,
                Some("Original"),
            ),
            (
                VoiceMode::Translate,
                VoiceIntentKind::TranslateSelection,
                Some("Original"),
            ),
            (VoiceMode::Translate, VoiceIntentKind::TranslateInsert, None),
            (
                VoiceMode::Ask,
                VoiceIntentKind::AskSelection,
                Some("Original"),
            ),
            (VoiceMode::Ask, VoiceIntentKind::OpenQuestion, None),
            (VoiceMode::Dictate, VoiceIntentKind::OpenQuestion, None),
        ] {
            assert!(!remote_output_is_final(&config, mode, kind, selected));
        }
        config.translate_enabled = true;
        assert!(!remote_output_is_final(
            &config,
            VoiceMode::Dictate,
            VoiceIntentKind::DictateInsert,
            None
        ));
    }
}

fn selected_text_command_requires_llm(selected_text: Option<&str>) -> bool {
    selected_text_has_content(selected_text)
}

fn voice_intent_requires_generated_output(kind: crate::voice_intent::VoiceIntentKind) -> bool {
    !matches!(
        kind,
        crate::voice_intent::VoiceIntentKind::DictateInsert
            | crate::voice_intent::VoiceIntentKind::Search
    )
}

fn history_provider_kind(config: &storage::AppConfig) -> storage::HistoryProviderKind {
    let provider = if config.polish_enabled {
        config.llm_provider.as_str()
    } else {
        config.stt_provider.as_str()
    };
    if provider == "cloud" {
        return storage::HistoryProviderKind::ManagedCloud;
    }
    if provider == "ollama"
        || provider == "apple-speech"
        || (provider == "custom-whisper"
            && (config.stt_custom_base_url.contains("localhost")
                || config.stt_custom_base_url.contains("127.0.0.1")))
    {
        return storage::HistoryProviderKind::Local;
    }
    storage::HistoryProviderKind::Byok
}

fn route_pipeline_voice_intent(
    mode: crate::voice_intent::VoiceMode,
    raw_text: &str,
    selected_text: Option<&str>,
    config: &storage::AppConfig,
) -> crate::voice_intent::VoiceIntent {
    let speech_language = if config.stt_language == "multi" {
        crate::voice_intent::SpeechLanguageMode::Automatic
    } else {
        crate::voice_intent::SpeechLanguageMode::Explicit(&config.stt_language)
    };
    crate::voice_intent::VoiceIntentRouter::route(crate::voice_intent::VoiceRouteRequest {
        mode,
        utterance: raw_text,
        has_selected_text: selected_text_has_content(selected_text),
        speech_language,
        flags: config.voice_routing_flags,
    })
}

fn streaming_insert_strategy_for_config(
    config: &storage::AppConfig,
    has_selected_text: bool,
    accessibility_trusted: bool,
    keyboard_available: bool,
) -> Option<output::InsertionStrategy> {
    if !config.streaming_insert_enabled || has_selected_text || !accessibility_trusted {
        return None;
    }

    let strategy = output::InsertionStrategy::from_config_value(&config.insertion_strategy)
        .direct_streaming_strategy()?;
    if strategy == output::InsertionStrategy::Keyboard && !keyboard_available {
        return None;
    }

    Some(strategy)
}

fn streaming_insert_strategy_for_runtime(
    config: &storage::AppConfig,
    selected_text: Option<&str>,
) -> Option<output::InsertionStrategy> {
    let keyboard_available = output::keyboard::check_keyboard_available().is_ok();
    streaming_insert_strategy_for_config(
        config,
        selected_text_has_content(selected_text),
        is_accessibility_trusted(),
        keyboard_available,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveSceneHistoryDiagnostics {
    id: Option<String>,
    source: Option<String>,
    name: Option<String>,
    prompt_chars: Option<i64>,
    prompt_truncated: bool,
}

fn active_scene_history_diagnostics(
    active_scene: Option<&storage::ActiveScene>,
) -> ActiveSceneHistoryDiagnostics {
    match active_scene {
        Some(scene) => {
            let sanitized_prompt = scene.prompt_template.replace('\0', "").trim().to_string();
            let prompt_chars = sanitized_prompt.chars().count();
            ActiveSceneHistoryDiagnostics {
                id: Some(scene.id.clone()),
                source: Some(scene.source.clone()),
                name: Some(scene.name.clone()),
                prompt_chars: Some(prompt_chars.min(storage::SCENE_PROMPT_MAX_CHARS) as i64),
                prompt_truncated: prompt_chars > storage::SCENE_PROMPT_MAX_CHARS,
            }
        }
        None => ActiveSceneHistoryDiagnostics {
            id: None,
            source: None,
            name: None,
            prompt_chars: None,
            prompt_truncated: false,
        },
    }
}

#[derive(Debug, Clone)]
struct StreamingInsertReport {
    strategy: output::InsertionStrategy,
    inserted_text: String,
    attempted_chunks: usize,
    failed: bool,
    target_lost: bool,
    error_message: Option<String>,
}

impl StreamingInsertReport {
    fn new(strategy: output::InsertionStrategy) -> Self {
        Self {
            strategy,
            inserted_text: String::new(),
            attempted_chunks: 0,
            failed: false,
            target_lost: false,
            error_message: None,
        }
    }

    fn chars_inserted(&self) -> usize {
        self.inserted_text.chars().count()
    }

    fn has_inserted_text(&self) -> bool {
        !self.inserted_text.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StreamingRecoveryAction {
    AlreadyComplete,
    InsertSuffix { suffix: String },
    CopyFullToClipboard { reason: String },
    CopyPartialToClipboard { reason: String },
    NoRecoveryNeeded,
}

fn streaming_recovery_action(
    report: &StreamingInsertReport,
    polished_text: Option<&str>,
    llm_succeeded: bool,
    target_still_trusted: bool,
) -> StreamingRecoveryAction {
    if !report.has_inserted_text() {
        return StreamingRecoveryAction::NoRecoveryNeeded;
    }

    if !llm_succeeded {
        return StreamingRecoveryAction::CopyPartialToClipboard {
            reason: "LLM failed after partial streaming insert".to_string(),
        };
    }

    let Some(polished_text) = polished_text else {
        return StreamingRecoveryAction::CopyFullToClipboard {
            reason: "missing polished text after successful LLM response".to_string(),
        };
    };

    if report.inserted_text == polished_text {
        return StreamingRecoveryAction::AlreadyComplete;
    }

    if polished_text.starts_with(&report.inserted_text) {
        if !target_still_trusted {
            return StreamingRecoveryAction::CopyFullToClipboard {
                reason: "target app changed after partial streaming insert".to_string(),
            };
        }

        let suffix: String = polished_text
            .chars()
            .skip(report.inserted_text.chars().count())
            .collect();
        return StreamingRecoveryAction::InsertSuffix { suffix };
    }

    StreamingRecoveryAction::CopyFullToClipboard {
        reason: "streaming prefix did not match final output".to_string(),
    }
}

struct StreamingInsertWorker {
    sender: mpsc::UnboundedSender<String>,
    handle: tokio::task::JoinHandle<StreamingInsertReport>,
}

impl StreamingInsertWorker {
    async fn finish(self) -> Option<StreamingInsertReport> {
        drop(self.sender);
        match self.handle.await {
            Ok(report) => Some(report),
            Err(error) => {
                tracing::warn!("Streaming insert worker failed to join: {error}");
                None
            }
        }
    }
}

fn spawn_streaming_insert_worker(context: StreamingInsertWorkerContext) -> StreamingInsertWorker {
    let (sender, receiver) = mpsc::unbounded_channel();
    let handle = tokio::spawn(run_streaming_insert_worker(context, receiver));
    StreamingInsertWorker { sender, handle }
}

struct StreamingInsertWorkerContext {
    app_handle: tauri::AppHandle,
    session: SessionClock,
    session_id: u64,
    context_detector: app_detector::ContextDetectorHandle,
    strategy: output::InsertionStrategy,
    windows_sendinput_options: output::windows_sendinput::WindowsSendInputOptions,
    expected_target_guard: TargetAppGuard,
    expected_target_label: String,
}

async fn run_streaming_insert_worker(
    context: StreamingInsertWorkerContext,
    mut receiver: mpsc::UnboundedReceiver<String>,
) -> StreamingInsertReport {
    let StreamingInsertWorkerContext {
        app_handle,
        session,
        session_id,
        context_detector,
        strategy,
        windows_sendinput_options,
        expected_target_guard,
        expected_target_label,
    } = context;
    let mut report = StreamingInsertReport::new(strategy);

    while let Some(chunk) = receiver.recv().await {
        if !session.is_live(session_id) {
            break;
        }
        if chunk.is_empty() {
            continue;
        }

        if !context_detector.target_still_matches_now(&expected_target_guard) {
            report.failed = true;
            report.target_lost = true;
            report.error_message = Some(format!(
                "Target app changed while streaming output for '{}'",
                expected_target_label
            ));
            break;
        }

        report.attempted_chunks += 1;
        match output::output_stream_chunk(&app_handle, &chunk, strategy, windows_sendinput_options)
            .await
        {
            Ok(insert_result)
                if insert_result.status == output::InsertStatus::Inserted
                    && insert_result.chars_inserted == chunk.chars().count() =>
            {
                report.inserted_text.push_str(&chunk);
            }
            Ok(insert_result) if insert_result.chars_inserted > 0 => {
                let partial_text: String =
                    chunk.chars().take(insert_result.chars_inserted).collect();
                report.inserted_text.push_str(&partial_text);
                report.failed = true;
                report.error_message = insert_result.message.or_else(|| {
                    Some(format!(
                        "Streaming insert stopped with status {:?}",
                        insert_result.status
                    ))
                });
                break;
            }
            Ok(insert_result) => {
                report.failed = true;
                report.error_message = insert_result.message.or_else(|| {
                    Some(format!(
                        "Streaming insert stopped with status {:?}",
                        insert_result.status
                    ))
                });
                break;
            }
            Err(error) => {
                report.failed = true;
                report.error_message = Some(error);
                break;
            }
        }
    }

    report
}

fn take_matching_stt_error(
    stt_error: &Mutex<Option<(u64, crate::error::UserError)>>,
    session_id: u64,
) -> Option<crate::error::UserError> {
    let mut guard = stt_error.lock().unwrap_or_else(|e| e.into_inner());
    if guard
        .as_ref()
        .is_some_and(|(latched_session_id, _)| *latched_session_id == session_id)
    {
        return guard.take().map(|(_, error)| error);
    }
    None
}

fn latch_stt_task_error_if_active(
    session: &SessionClock,
    stt_error: &Mutex<Option<(u64, crate::error::UserError)>>,
    task_session_id: u64,
    error: &crate::error::AppError,
) -> Option<crate::error::UserError> {
    if !session.is_live(task_session_id) {
        return None;
    }
    let user_error = error.to_user_error();
    *stt_error.lock().unwrap_or_else(|e| e.into_inner()) =
        Some((task_session_id, user_error.clone()));
    Some(user_error)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PipelineStartOptions {
    pub force_translate: bool,
    /// Type recognised words into the target app while the user speaks (no AI polish).
    pub live_typing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranslationOperationPhase {
    Capturing,
    Finalizing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TranslationOperationState {
    target: String,
    phase: TranslationOperationPhase,
}

impl TranslationOperationState {
    fn new(target: String) -> Self {
        Self {
            target,
            phase: TranslationOperationPhase::Capturing,
        }
    }

    fn switch_target(&mut self, target: String) -> std::result::Result<String, &'static str> {
        if self.phase != TranslationOperationPhase::Capturing {
            return Err("translation_operation_finished");
        }
        Ok(std::mem::replace(&mut self.target, target))
    }

    fn finalize(&mut self) -> String {
        self.phase = TranslationOperationPhase::Finalizing;
        self.target.clone()
    }
}

fn apply_pipeline_start_options(
    mut config: storage::AppConfig,
    options: PipelineStartOptions,
) -> storage::AppConfig {
    if options.force_translate {
        config.translate_enabled = true;
    }
    config
}

#[derive(Clone)]
pub struct PipelineHandle {
    app_handle: tauri::AppHandle,
    context_detector: app_detector::ContextDetectorHandle,
    phases: Phases,
    escape_event_pending: Arc<AtomicBool>,
    enter_event_pending: Arc<AtomicBool>,
    audio_handle: Arc<Mutex<Option<AudioCaptureHandle>>>,
    audio_volume: Arc<Mutex<f32>>,
    accumulated_text: Arc<Mutex<String>>,
    stt_session: Arc<Mutex<Option<SttTaskControl>>>,
    stt_error: Arc<Mutex<Option<(u64, crate::error::UserError)>>>,
    session: SessionClock,
    preloaded_config: Arc<Mutex<Option<storage::AppConfig>>>,
    preloaded_app_ctx: Arc<Mutex<Option<RecordingContext>>>,
    preloaded_dictionary: Arc<Mutex<Option<Vec<String>>>>,
    preloaded_correction_rules: Arc<Mutex<Option<Vec<llm::CorrectionRule>>>>,
    preloaded_selected_text: Arc<Mutex<Option<String>>>,
    preloaded_voice_mode: Arc<Mutex<Option<crate::voice_intent::VoiceMode>>>,
    cloud_operation_id: Arc<Mutex<Option<String>>>,
    recording_start: Arc<Mutex<Option<std::time::Instant>>>,
    active_translation_operation: Arc<Mutex<Option<TranslationOperationState>>>,
    /// Live typing session: words are typed while recording, and stop() skips polish.
    live_typing: Arc<AtomicBool>,
    live_insert_worker: Arc<Mutex<Option<StreamingInsertWorker>>>,
    shared_client: reqwest::Client,
    /// Serializes start()/stop() so that stop() waits for start() to finish
    /// its setup before reading shared state (preloaded_config, audio_handle, etc.).
    /// Without this, a quick press-release in hold mode causes stop() to run
    /// while start() is still connecting to STT, finding empty fields.
    pipeline_lock: Arc<tokio::sync::Mutex<()>>,
}

struct PolishTextInput<'a> {
    raw_text: &'a str,
    voice_mode: crate::voice_intent::VoiceMode,
    config: &'a storage::AppConfig,
    app_ctx: &'a RecordingContext,
    dictionary_words: Vec<String>,
    correction_rules: Vec<llm::CorrectionRule>,
    selected_text: Option<String>,
    session_token: String,
    operation_id: Option<String>,
    voice_intent: crate::voice_intent::VoiceIntent,
    popup_fallback_enabled: bool,
    session_generation: u64,
}

#[derive(Debug, Clone)]
struct PolishTextOutcome {
    final_text: String,
    llm_elapsed: std::time::Duration,
    history_output_status: Option<String>,
    history_output_error: Option<String>,
    voice_execution: Option<crate::voice_intent::executor::VoiceExecutionResult>,
}

pub(crate) struct AskVoiceDraftOutcome {
    pub text: String,
    pub execution: crate::voice_intent::executor::VoiceExecutionResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HistoryOutputMetadata {
    status: Option<String>,
    error: Option<String>,
}

fn history_output_metadata_for_insert_result(
    insert_result: &output::InsertResult,
    intentional_clipboard_copy: bool,
) -> HistoryOutputMetadata {
    let status = match insert_result.status {
        output::InsertStatus::PartiallyInserted => "partial",
        output::InsertStatus::Failed => "failed",
        output::InsertStatus::CopiedFallback
            if intentional_clipboard_copy && insert_result.warning_code.is_none() =>
        {
            "copied"
        }
        output::InsertStatus::CopiedFallback => "clipboard_fallback",
        output::InsertStatus::Inserted if insert_result.warning_code.is_some() => "fallback",
        output::InsertStatus::Inserted => "inserted",
    };

    HistoryOutputMetadata {
        status: Some(status.to_string()),
        error: insert_result.message.clone(),
    }
}

fn history_output_metadata_for_output_result(
    output_result: std::result::Result<&output::InsertResult, &anyhow::Error>,
    intentional_clipboard_copy: bool,
) -> HistoryOutputMetadata {
    match output_result {
        Ok(insert_result) => {
            history_output_metadata_for_insert_result(insert_result, intentional_clipboard_copy)
        }
        Err(error) => HistoryOutputMetadata {
            status: Some("failed".to_string()),
            error: Some(error.to_string()),
        },
    }
}

fn history_output_metadata_with_prior_error(
    output_result: std::result::Result<&output::InsertResult, &anyhow::Error>,
    intentional_clipboard_copy: bool,
    prior_error: &str,
) -> HistoryOutputMetadata {
    let mut metadata =
        history_output_metadata_for_output_result(output_result, intentional_clipboard_copy);
    if matches!(metadata.status.as_deref(), Some("inserted" | "copied")) {
        metadata.status = Some("fallback".to_string());
    }
    metadata.error = Some(match metadata.error {
        Some(output_error) => format!("{prior_error}; raw output: {output_error}"),
        None => format!("{prior_error}; output raw text"),
    });
    metadata
}

struct PipelineVoiceExecutionBackend<'a> {
    pipeline: &'a PipelineHandle,
    app_name: &'a str,
    question: &'a str,
    intent_kind: crate::voice_intent::VoiceIntentKind,
    target_guard: &'a TargetAppGuard,
    config: &'a storage::AppConfig,
    already_copied: bool,
    popup_fallback_enabled: bool,
    session_generation: u64,
}

#[async_trait::async_trait]
impl crate::voice_intent::executor::VoiceExecutionBackend for PipelineVoiceExecutionBackend<'_> {
    fn target_matches(&mut self, guard: &TargetAppGuard) -> bool {
        self.pipeline
            .context_detector
            .target_still_matches_now(guard)
    }

    async fn restore_target(
        &mut self,
        guard: &TargetAppGuard,
    ) -> std::result::Result<bool, String> {
        if let Some(window) = self.pipeline.app_handle.get_webview_window("ask") {
            let _ = window.hide();
        }
        let detector = self.pipeline.context_detector.clone();
        let guard = guard.clone();
        tokio::task::spawn_blocking(move || detector.restore_target_application(&guard))
            .await
            .map_err(|error| error.to_string())
    }

    async fn insert_at_cursor(&mut self, text: &str) -> std::result::Result<(), String> {
        let result = self
            .pipeline
            .output_text(
                text,
                self.app_name,
                self.target_guard,
                self.config,
                self.session_generation,
            )
            .await
            .map_err(|error| error.to_string())?;
        if result.status == output::InsertStatus::Inserted {
            Ok(())
        } else if result.status == output::InsertStatus::CopiedFallback {
            self.already_copied = true;
            Err("output copied instead of inserted".to_string())
        } else {
            Err("output was not inserted".to_string())
        }
    }

    async fn replace_selection(&mut self, text: &str) -> std::result::Result<(), String> {
        self.insert_at_cursor(text).await
    }

    async fn popup_answer(&mut self, text: &str) -> std::result::Result<(), String> {
        if !self.popup_fallback_enabled {
            return Err("popup fallback is owned by the Ask caller".to_string());
        }
        crate::commands::ask::show_answer_window_with_metadata(
            &self.pipeline.app_handle,
            self.question.to_string(),
            text.to_string(),
            self.intent_kind,
            true,
            false,
        )
    }

    async fn copy_to_clipboard(&mut self, text: &str) -> std::result::Result<(), String> {
        if self.already_copied {
            return Ok(());
        }
        let mut copy_config = self.config.clone();
        copy_config.insertion_strategy = "clipboardCopyOnly".to_string();
        let result = self
            .pipeline
            .output_text(
                text,
                self.app_name,
                &TargetAppGuard::default(),
                &copy_config,
                self.session_generation,
            )
            .await
            .map_err(|error| error.to_string())?;
        if result.status == output::InsertStatus::CopiedFallback {
            self.already_copied = true;
            Ok(())
        } else {
            Err("clipboard copy did not complete".to_string())
        }
    }

    async fn open_search(
        &mut self,
        _url: &crate::voice_intent::search::SearchUrl,
    ) -> std::result::Result<(), String> {
        Err("search is not available in the dictation pipeline".to_string())
    }
}

impl PolishTextOutcome {
    fn normal(final_text: String, llm_elapsed: std::time::Duration) -> Self {
        Self {
            final_text,
            llm_elapsed,
            history_output_status: None,
            history_output_error: None,
            voice_execution: None,
        }
    }

    fn with_history_status(
        final_text: String,
        llm_elapsed: std::time::Duration,
        status: &'static str,
        error: impl Into<String>,
    ) -> Self {
        Self {
            final_text,
            llm_elapsed,
            history_output_status: Some(status.to_string()),
            history_output_error: Some(error.into()),
            voice_execution: None,
        }
    }

    fn with_execution(
        final_text: String,
        llm_elapsed: std::time::Duration,
        execution: crate::voice_intent::executor::VoiceExecutionResult,
        history_output_status: Option<String>,
        history_output_error: Option<String>,
    ) -> Self {
        Self {
            final_text,
            llm_elapsed,
            history_output_status,
            history_output_error,
            voice_execution: Some(execution),
        }
    }
}

fn consume_escape_event(pending: &AtomicBool, shortcut: &Shortcut) -> bool {
    shortcut.key == Code::Escape
        && shortcut.mods.is_empty()
        && pending.swap(false, Ordering::SeqCst)
}

fn consume_enter_event(pending: &AtomicBool, shortcut: &Shortcut) -> bool {
    shortcut.key == Code::Enter && shortcut.mods.is_empty() && pending.swap(false, Ordering::SeqCst)
}

fn recording_control_key(
    shortcut: &Shortcut,
    state: PipelineState,
    ask_busy: bool,
) -> Option<Code> {
    if !shortcut.mods.is_empty() {
        return None;
    }
    match shortcut.key {
        Code::Escape if state.holds_escape() || ask_busy => Some(Code::Escape),
        Code::Enter if state.holds_enter() => Some(Code::Enter),
        _ => None,
    }
}

/// Tauri adapter for the session's screen-side effects: capsule events, the tray and the
/// global Escape/Enter shortcuts that exist only while a session needs them.
struct TauriSessionNotifier {
    app_handle: tauri::AppHandle,
    state: StateCell,
    escape_registered: AtomicBool,
    enter_registered: AtomicBool,
}

impl TauriSessionNotifier {
    fn new(app_handle: tauri::AppHandle, state: StateCell) -> Self {
        Self {
            app_handle,
            state,
            escape_registered: AtomicBool::new(false),
            enter_registered: AtomicBool::new(false),
        }
    }

    fn ask_busy(&self) -> bool {
        self.app_handle
            .try_state::<crate::commands::ask::AskDictationState>()
            .is_some_and(|state| state.is_busy())
    }
}

impl SessionNotifier for TauriSessionNotifier {
    fn hold_escape(&self, hold: bool) {
        let escape = Shortcut::new(None, Code::Escape);
        if !hold {
            if self.escape_registered.swap(false, Ordering::SeqCst) {
                if let Err(error) = self.app_handle.global_shortcut().unregister(escape) {
                    tracing::warn!("Failed to release recording Escape shortcut: {error}");
                }
            }
            return;
        }
        if self.escape_registered.load(Ordering::SeqCst)
            || self.app_handle.global_shortcut().is_registered(escape)
        {
            return;
        }
        let registered = self
            .app_handle
            .global_shortcut()
            .on_shortcut(escape, |app, _, event| {
                let pipeline = app.state::<PipelineHandle>();
                // The plugin calls its global handler after this per-shortcut handler.
                pipeline.escape_event_pending.store(true, Ordering::SeqCst);
                if event.state != ShortcutState::Pressed {
                    return;
                }
                let ask_busy = app
                    .try_state::<crate::commands::ask::AskDictationState>()
                    .is_some_and(|state| state.is_busy());
                if pipeline.current_state() == PipelineState::Idle && !ask_busy {
                    return;
                }
                let generation = pipeline.session.current();
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let pipeline = app.state::<PipelineHandle>();
                    if let Some(ask) = app
                        .try_state::<crate::commands::ask::AskDictationState>()
                        .filter(|state| state.is_busy())
                    {
                        let _ = crate::commands::ask::abort_ask_dictation(app.clone(), ask);
                    } else if pipeline.session.is_live(generation) {
                        pipeline.abort();
                    }
                });
            });
        match registered {
            Ok(()) => {
                self.escape_registered.store(true, Ordering::SeqCst);
                if !self.state.get().holds_escape()
                    && !self.ask_busy()
                    && self.escape_registered.swap(false, Ordering::SeqCst)
                {
                    if let Err(error) = self.app_handle.global_shortcut().unregister(escape) {
                        tracing::warn!("Failed to release recording Escape shortcut: {error}");
                    }
                }
            }
            Err(error) => tracing::warn!("Recording Escape shortcut is unavailable: {error}"),
        }
    }

    fn hold_enter(&self, hold: bool) {
        let enter = Shortcut::new(None, Code::Enter);
        if !hold {
            if self.enter_registered.swap(false, Ordering::SeqCst) {
                if let Err(error) = self.app_handle.global_shortcut().unregister(enter) {
                    tracing::warn!("Failed to release recording Enter shortcut: {error}");
                }
            }
            return;
        }
        if self.enter_registered.load(Ordering::SeqCst)
            || self.app_handle.global_shortcut().is_registered(enter)
        {
            return;
        }
        let registered = self
            .app_handle
            .global_shortcut()
            .on_shortcut(enter, |app, _, event| {
                let pipeline = app.state::<PipelineHandle>();
                // The plugin calls its global handler after this per-shortcut handler.
                pipeline.enter_event_pending.store(true, Ordering::SeqCst);
                if event.state != ShortcutState::Pressed
                    || pipeline.current_state() != PipelineState::Recording
                {
                    return;
                }
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let pipeline = app.state::<PipelineHandle>();
                    if let Err(error) = pipeline.stop().await {
                        tracing::warn!("Enter could not finish the recording: {error}");
                    }
                });
            });
        match registered {
            Ok(()) => {
                self.enter_registered.store(true, Ordering::SeqCst);
                if !self.state.get().holds_enter()
                    && self.enter_registered.swap(false, Ordering::SeqCst)
                {
                    if let Err(error) = self.app_handle.global_shortcut().unregister(enter) {
                        tracing::warn!("Failed to release recording Enter shortcut: {error}");
                    }
                }
            }
            Err(error) => tracing::warn!("Recording Enter shortcut is unavailable: {error}"),
        }
    }

    fn phase_entered(&self, state: PipelineState) {
        if state == PipelineState::Idle {
            let _ = self.app_handle.emit(
                "pipeline:voice_mode",
                Option::<crate::voice_intent::VoiceMode>::None,
            );
        }
        let _ = self.app_handle.emit("pipeline:state", state);

        // Update tray tooltip + menu to reflect pipeline state
        if let Some(tray_handle) = self.app_handle.try_state::<crate::TrayHandle>() {
            let tooltip = match state {
                PipelineState::Preparing => "Ultra STT - Preparing...",
                PipelineState::Recording => "Ultra STT - Recording...",
                PipelineState::Transcribing => "Ultra STT - Transcribing...",
                PipelineState::Polishing => "Ultra STT - Polishing...",
                PipelineState::Outputting => "Ultra STT - Outputting...",
                PipelineState::AskRecording => "Ultra STT - Ask...",
                PipelineState::AskThinking => "Ultra STT - Answering...",
                PipelineState::Idle => "Ultra STT",
            };
            if let Ok(t) = tray_handle.tray.lock() {
                let _ = t.set_tooltip(Some(tooltip));
            }
        }
        crate::refresh_tray(&self.app_handle);
    }
}

impl PipelineHandle {
    pub fn new(
        app_handle: tauri::AppHandle,
        shared_client: reqwest::Client,
        context_detector: app_detector::ContextDetectorHandle,
    ) -> Self {
        let state = StateCell::new();
        let notifier = Arc::new(TauriSessionNotifier::new(app_handle.clone(), state.clone()));
        Self {
            app_handle,
            context_detector,
            phases: Phases::new(state, notifier),
            escape_event_pending: Arc::new(AtomicBool::new(false)),
            enter_event_pending: Arc::new(AtomicBool::new(false)),
            audio_handle: Arc::new(Mutex::new(None)),
            audio_volume: Arc::new(Mutex::new(0.0)),
            accumulated_text: Arc::new(Mutex::new(String::new())),
            stt_session: Arc::new(Mutex::new(None)),
            stt_error: Arc::new(Mutex::new(None)),
            session: SessionClock::new(),
            preloaded_config: Arc::new(Mutex::new(None)),
            preloaded_app_ctx: Arc::new(Mutex::new(None)),
            preloaded_dictionary: Arc::new(Mutex::new(None)),
            preloaded_correction_rules: Arc::new(Mutex::new(None)),
            preloaded_selected_text: Arc::new(Mutex::new(None)),
            preloaded_voice_mode: Arc::new(Mutex::new(None)),
            cloud_operation_id: Arc::new(Mutex::new(None)),
            recording_start: Arc::new(Mutex::new(None)),
            active_translation_operation: Arc::new(Mutex::new(None)),
            live_typing: Arc::new(AtomicBool::new(false)),
            live_insert_worker: Arc::new(Mutex::new(None)),
            shared_client,
            pipeline_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Syncs Escape for an Ask phase, which is not stored in this session yet.
    pub(crate) fn sync_escape_shortcut(&self, state: PipelineState) {
        self.phases.hold_escape_for(state);
    }

    fn set_state(&self, new_state: PipelineState) {
        if new_state == PipelineState::Idle {
            *self
                .active_translation_operation
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = None;
        }
        self.phases.enter(new_state);
    }

    pub(crate) fn take_escape_event_for_global(&self, shortcut: &Shortcut) -> bool {
        consume_escape_event(&self.escape_event_pending, shortcut)
            || consume_enter_event(&self.enter_event_pending, shortcut)
    }

    pub(crate) fn route_recording_control(
        &self,
        shortcut: &Shortcut,
        state: ShortcutState,
    ) -> bool {
        let ask_busy = self
            .app_handle
            .try_state::<crate::commands::ask::AskDictationState>()
            .is_some_and(|ask| ask.is_busy());
        let action = recording_control_key(shortcut, self.current_state(), ask_busy);
        if action == Some(Code::Escape) {
            if state == ShortcutState::Pressed {
                if let Some(ask) = self
                    .app_handle
                    .try_state::<crate::commands::ask::AskDictationState>()
                    .filter(|ask| ask.is_busy())
                {
                    let _ = crate::commands::ask::abort_ask_dictation(self.app_handle.clone(), ask);
                } else {
                    self.abort();
                }
            }
            return true;
        }
        if action == Some(Code::Enter) {
            if state == ShortcutState::Pressed {
                let pipeline = self.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = pipeline.stop().await {
                        tracing::warn!("Recording Enter stop failed: {error}");
                    }
                });
            }
            return true;
        }
        false
    }

    pub fn current_state(&self) -> PipelineState {
        self.phases.get()
    }

    pub(crate) fn switch_active_translation_target(
        &self,
        target: String,
    ) -> std::result::Result<String, String> {
        if self.current_state() != PipelineState::Recording {
            return Err("translation_operation_finished".to_string());
        }
        let mut operation = self
            .active_translation_operation
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        operation
            .as_mut()
            .ok_or_else(|| "translation_not_recording".to_string())?
            .switch_target(target)
            .map_err(str::to_string)
    }

    /// Immediately abort the pipeline regardless of current state.
    /// Stops audio capture, forces state to Idle, and ends the session so any
    /// ongoing start()/stop() and background task stands down.
    pub fn abort(&self) {
        tracing::info!(
            "Pipeline abort requested (current state: {:?})",
            self.current_state()
        );

        // End the session so any running start()/stop() and STT task exits early
        self.session.end();

        // Stop audio capture (closes channel → STT task terminates naturally)
        {
            let mut handle = self.audio_handle.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref mut h) = *handle {
                h.stop();
            }
            *handle = None;
        }
        if let Some(control) = self
            .stt_session
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            control.abort.notify_one();
            control.done.notify_one();
            if let Err(error) = control
                .journal
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .discard()
            {
                tracing::error!("Could not remove cancelled recording backup: {error}");
            }
        }

        // Clear accumulated text
        self.accumulated_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        *self.stt_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .cloud_operation_id
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;

        // Force state to Idle — emits pipeline:state event to sync frontend
        self.set_state(PipelineState::Idle);
    }

    fn clear_stt_session(&self, session_id: u64) {
        let mut session = self.stt_session.lock().unwrap_or_else(|e| e.into_inner());
        if session.as_ref().map(|control| control.id) == Some(session_id) {
            *session = None;
        }
    }

    /// Capture selected text from the foreground app by simulating Ctrl+C / Cmd+C.
    /// Must be called when no hotkey modifier keys are physically held down.
    /// Called from async context via block_in_place, so std::thread::sleep is acceptable.
    fn capture_selected_text(&self) -> Option<String> {
        crate::selection::capture_selected_text()
    }

    async fn load_config(&self) -> storage::AppConfig {
        self.app_handle
            .state::<storage::ConfigManager>()
            .load()
            .await
            .unwrap_or_default()
    }

    pub async fn start(&self) -> Result<()> {
        self.start_with_options(PipelineStartOptions::default())
            .await
    }

    pub async fn start_with_options(&self, options: PipelineStartOptions) -> Result<()> {
        // Hold pipeline_lock for the entire setup so stop() cannot read
        // partially-initialised state (preloaded_config, audio_handle, etc.).
        let _guard = self.pipeline_lock.lock().await;

        // Atomic CAS: only one caller can transition Idle → Preparing. Recording is emitted only
        // after audio capture is ready, so the capsule does not tell users to speak too early.
        if !self
            .phases
            .claim(PipelineState::Idle, PipelineState::Preparing)
        {
            return Ok(());
        }
        let session_id = self.session.begin();
        crate::capsule_window::capture_recording_monitor(&self.app_handle);
        self.set_state(PipelineState::Preparing);
        self.live_typing
            .store(options.live_typing, Ordering::SeqCst);
        *self
            .live_insert_worker
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;

        // Clear accumulated text
        self.accumulated_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        *self.stt_error.lock().unwrap_or_else(|e| e.into_inner()) = None;

        // P0-2: Load config BEFORE starting audio capture — fail fast on missing API key
        let config_data = apply_pipeline_start_options(self.load_config().await, options);
        let voice_mode = if options.force_translate {
            crate::voice_intent::VoiceMode::Translate
        } else {
            crate::voice_intent::VoiceMode::Dictate
        };
        *self
            .preloaded_voice_mode
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(voice_mode);
        *self
            .preloaded_config
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(config_data.clone());
        *self
            .preloaded_app_ctx
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(
            self.context_detector
                .snapshot_for_recording_enabled(config_data.context_adaptation_enabled),
        );
        let dictionary_store = self.app_handle.state::<storage::DictionaryStore>();
        let dict_words = dictionary_store.words().await;
        let correction_rules = dictionary_store
            .enabled_correction_rules()
            .await
            .into_iter()
            .map(|rule| llm::CorrectionRule {
                id: rule.id,
                pattern: rule.pattern,
                replacement: rule.replacement,
                enabled: rule.enabled,
            })
            .collect::<Vec<_>>();
        // Live typing has no AI polish: the dictionary guides recognition and correction
        // rules are applied before each word is typed.
        let live_hints = options.live_typing.then(|| {
            let corrections = correction_rules
                .iter()
                .map(|rule| (rule.pattern.clone(), rule.replacement.clone()))
                .collect::<Vec<_>>();
            (dict_words.clone(), corrections)
        });
        *self
            .preloaded_dictionary
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(dict_words);
        *self
            .preloaded_correction_rules
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(correction_rules);

        let stt_api_key = if config_data.stt_provider == stt::config::CUSTOM_WHISPER_PROVIDER
            && config_data.stt_custom_preset == "ultra-local"
        {
            String::new()
        } else if config_data.stt_provider == "cloud" {
            self.app_handle
                .state::<SessionTokenStore>()
                .0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            match resolve_stt_config_secret(&config_data, &SystemCredentialVault) {
                Ok(secret) => secret,
                Err(error) => {
                    tracing::warn!("Failed to read STT credential: {error}");
                    let _ = self.app_handle.emit(
                        "pipeline:error",
                        "Failed to read STT credential from the system vault.",
                    );
                    *self
                        .preloaded_config
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = None;
                    *self
                        .preloaded_app_ctx
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = None;
                    *self
                        .preloaded_dictionary
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = None;
                    *self
                        .preloaded_correction_rules
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = None;
                    self.set_state(PipelineState::Idle);
                    return Ok(());
                }
            }
        };

        tracing::debug!(
            "Pipeline using config: stt_provider={}, stt_key_len={}, stt_lang={}",
            config_data.stt_provider,
            stt_api_key.len(),
            config_data.stt_language
        );

        // Guard: empty API key - bail before starting audio when provider requires one.
        if stt::config::stt_provider_requires_api_key(&config_data.stt_provider)
            && stt_api_key.is_empty()
        {
            let _ = self.app_handle.emit(
                "pipeline:error",
                "STT API key is not configured. Please set it in Settings -> Speech Recognition.",
            );
            *self
                .preloaded_config
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_app_ctx
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_dictionary
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_correction_rules
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            self.set_state(PipelineState::Idle);
            return Ok(());
        }

        let custom_whisper_config =
            if config_data.stt_provider == stt::config::CUSTOM_WHISPER_PROVIDER {
                match stt::config::build_custom_whisper_for_app(&config_data) {
                    Ok(cfg) => Some(cfg),
                    Err(e) => {
                        let _ = self.app_handle.emit("pipeline:error", e);
                        *self
                            .preloaded_config
                            .lock()
                            .unwrap_or_else(|e| e.into_inner()) = None;
                        *self
                            .preloaded_app_ctx
                            .lock()
                            .unwrap_or_else(|e| e.into_inner()) = None;
                        *self
                            .preloaded_dictionary
                            .lock()
                            .unwrap_or_else(|e| e.into_inner()) = None;
                        *self
                            .preloaded_correction_rules
                            .lock()
                            .unwrap_or_else(|e| e.into_inner()) = None;
                        self.set_state(PipelineState::Idle);
                        return Ok(());
                    }
                }
            } else {
                None
            };

        // Prepare STT configuration before starting the shared audio/STT readiness phase.
        let cloud_operation_id = generate_cloud_operation_id();
        *self
            .cloud_operation_id
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(cloud_operation_id.clone());

        let stt_config = SttConfig {
            api_key: stt_api_key,
            language: if config_data.stt_language == "multi" {
                None
            } else {
                Some(config_data.stt_language.clone())
            },
            smart_format: true,
            sample_rate: 16000,
            resource_id: if config_data.stt_provider == stt::volcengine::VOLCENGINE_DOUBAO_PROVIDER
            {
                Some(config_data.stt_volcengine_resource_id.clone())
            } else {
                None
            },
            operation_id: Some(cloud_operation_id),
            managed_audio: stt::capabilities::managed_audio_encoding_config(
                &config_data,
                chrono::Utc::now().timestamp(),
            ),
            provider_region: (config_data.stt_provider
                == stt::aliyun_qwen3_asr::ALIYUN_QWEN3_ASR_PROVIDER)
                .then(|| config_data.stt_aliyun_qwen_region.clone()),
        };
        let managed_cloud_session_token =
            (config_data.stt_provider == "cloud").then(|| stt_config.api_key.clone());
        if let Some(session_token) = managed_cloud_session_token.clone() {
            stt::cloud::warm_managed_cloud_on_intent(self.shared_client.clone(), session_token);
        }

        let provider_result = if let Some((dictionary, corrections)) = live_hints {
            // Clients of the owner's recognition PC (Mac, home) run live passes there.
            let backend = match custom_whisper_config.as_ref() {
                Some(remote)
                    if config_data.stt_custom_preset == stt::config::ULTRA_REMOTE_STT_PRESET =>
                {
                    stt::ultra_local::LiveBackend::Remote {
                        endpoint: stt::ultra_local::live_endpoint_for(&remote.endpoint),
                        token: stt_config.api_key.clone(),
                        client: self.shared_client.clone(),
                    }
                }
                _ => stt::ultra_local::LiveBackend::Local,
            };
            Ok(Box::new(stt::ultra_local::UltraLocalLiveProvider::new(
                &dictionary,
                corrections,
                backend,
                config_data.live_idle_stop_seconds as usize * 1000,
            )) as Box<dyn stt::SttProvider>)
        } else {
            stt::create_provider(
                &config_data.stt_provider,
                custom_whisper_config,
                Some(self.shared_client.clone()),
            )
        };
        let mut provider = match provider_result {
            Ok(provider) => provider,
            Err(e) => {
                tracing::error!("STT provider creation failed: {}", e);
                let _ = self
                    .app_handle
                    .emit("pipeline:error", format!("STT configuration failed: {e}"));
                *self
                    .preloaded_config
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_app_ctx
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_dictionary
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_correction_rules
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                self.set_state(PipelineState::Idle);
                return Ok(());
            }
        };
        // Start the platform audio backend before connecting STT. Both readiness
        // operations are then polled concurrently, so speech captured while a
        // network provider connects remains queued instead of being clipped.
        let config = AudioConfig::default();
        let (mut handle, mut audio_rx) = match AudioCaptureHandle::start(config) {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("Audio capture failed: {}", e);
                let _ = self
                    .app_handle
                    .emit("pipeline:error", format!("Audio capture failed: {e}"));
                *self
                    .preloaded_config
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_app_ctx
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_dictionary
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_correction_rules
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                self.set_state(PipelineState::Idle);
                return Ok(());
            }
        };
        let startup_result = crate::audio::await_recording_startup(
            handle.wait_until_ready(),
            provider.connect(&stt_config),
        )
        .await;
        let capture_ready_at = match startup_result {
            Ok(capture_ready_at) => capture_ready_at,
            Err(error) => {
                let message = match error {
                    crate::audio::RecordingStartupError::Audio(error) => {
                        format!("Audio capture failed: {error}")
                    }
                    crate::audio::RecordingStartupError::Stt(error) => {
                        format!("STT connection failed: {error}")
                    }
                    crate::audio::RecordingStartupError::Timeout => {
                        "Recording startup timed out after 30 seconds. Please try again."
                            .to_string()
                    }
                };
                tracing::error!("Recording startup failed: {}", message);
                handle.stop();
                let _ = self.app_handle.emit("pipeline:error", message);
                *self
                    .preloaded_config
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_app_ctx
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_dictionary
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                *self
                    .preloaded_correction_rules
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                self.set_state(PipelineState::Idle);
                return Ok(());
            }
        };

        // Store the audio handle's volume reference.
        // Check for a cancel first — if abort() was called while we were connecting
        // to STT, don't store the handle (it would be orphaned with nobody to stop it).
        if !self.session.is_live(session_id) {
            tracing::info!("Pipeline aborted during setup, discarding audio capture");
            // handle drops here, stopping the capture thread
            *self
                .preloaded_config
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_app_ctx
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_dictionary
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_correction_rules
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            self.set_state(PipelineState::Idle);
            return Ok(());
        }
        let audio_vol = handle.get_volume();
        *self.audio_volume.lock().unwrap_or_else(|e| e.into_inner()) = audio_vol;
        *self.audio_handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
        if !self.session.is_live(session_id) {
            tracing::info!("Pipeline aborted after storing audio capture, stopping capture");
            {
                let mut handle = self.audio_handle.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ref mut h) = *handle {
                    h.stop();
                }
                *handle = None;
            }
            *self
                .preloaded_config
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_app_ctx
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_dictionary
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_correction_rules
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            self.set_state(PipelineState::Idle);
            return Ok(());
        }

        let resolved_limit = stt::capabilities::resolve_recording_limit(
            &config_data,
            None,
            chrono::Utc::now().timestamp(),
        );
        let effective_max_seconds = provider
            .recording_limit_override_seconds()
            .map_or(resolved_limit.effective_max_seconds, |override_seconds| {
                resolved_limit.effective_max_seconds.min(override_seconds)
            });
        let deadline_explanation_key = provider
            .recording_limit_override_explanation_key()
            .unwrap_or(&resolved_limit.capability.explanation_key)
            .to_string();
        let recording_deadline = crate::recording_deadline::RecordingDeadline::new(
            session_id,
            crate::recording_deadline::RecordingKind::Dictation,
            capture_ready_at,
            effective_max_seconds,
        );
        if let Some(session_token) = managed_cloud_session_token {
            let warmup_client = self.shared_client.clone();
            let warmup_scope = self.session.recording_scope(session_id, &self.phases);
            tokio::spawn(async move {
                for elapsed_seconds in [240u64, 480] {
                    if elapsed_seconds >= u64::from(effective_max_seconds) {
                        break;
                    }
                    tokio::time::sleep_until(tokio::time::Instant::from_std(
                        capture_ready_at.monotonic
                            + std::time::Duration::from_secs(elapsed_seconds),
                    ))
                    .await;
                    if !warmup_scope.is_active() {
                        break;
                    }
                    stt::cloud::warm_managed_cloud_on_intent(
                        warmup_client.clone(),
                        session_token.clone(),
                    );
                }
            });
        }
        *self
            .recording_start
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(capture_ready_at.monotonic);
        *self
            .active_translation_operation
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = options
            .force_translate
            .then(|| TranslationOperationState::new(config_data.translation.active_target.clone()));
        // A cancel can land after the checks above; only a still-preparing session may record.
        if !self
            .phases
            .advance(PipelineState::Preparing, PipelineState::Recording)
        {
            tracing::info!("Pipeline aborted before recording began");
            *self
                .active_translation_operation
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = None;
            *self
                .preloaded_config
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_app_ctx
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_dictionary
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_correction_rules
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            return Ok(());
        }
        let _ = self.app_handle.emit("pipeline:voice_mode", voice_mode);
        let _ = self
            .app_handle
            .emit("pipeline:live_typing", options.live_typing);
        let _ = self
            .app_handle
            .emit("recording:deadline", recording_deadline.event);

        // Volume monitoring task
        let app_handle = self.app_handle.clone();
        let audio_handle_ref = self.audio_handle.clone();
        let state_ref = self.phases.clone();
        let volume_session = self.session.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(VOLUME_POLL_INTERVAL_MS)).await;
                if state_ref.get() != PipelineState::Recording {
                    break;
                }
                let (vol, capture_error) = audio_handle_ref
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                    .map(|handle| (handle.get_volume(), handle.runtime_error()))
                    .unwrap_or((0.0, None));
                if let Some(message) = capture_error {
                    let pipeline = app_handle.state::<PipelineHandle>();
                    let error = crate::error::AppError::Config(format!(
                        "Audio capture interrupted: {message}"
                    ));
                    if let Some(user_error) = latch_stt_task_error_if_active(
                        &volume_session,
                        pipeline.stt_error.as_ref(),
                        session_id,
                        &error,
                    ) {
                        let _ = app_handle.emit("pipeline:error", user_error);
                    }
                    if let Err(error) = pipeline.stop().await {
                        tracing::error!("Could not stop interrupted recording: {error}");
                    }
                    break;
                }
                let _ = app_handle.emit("audio:volume", vol);
            }
        });

        // Selected text will be captured in stop() after hotkey is released,
        // so Ctrl+C simulation won't conflict with held keys.
        *self
            .preloaded_selected_text
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;

        // STT streaming task — provider is already connected
        let app_handle = self.app_handle.clone();
        let accumulated = self.accumulated_text.clone();
        let journal = match self
            .app_handle
            .state::<crate::recording_archive::RecordingArchive>()
            .begin()
        {
            Ok(journal) => Arc::new(Mutex::new(journal)),
            Err(error) => {
                self.abort();
                let _ = self.app_handle.emit(
                    "pipeline:error",
                    crate::error::AppError::Config(format!(
                        "Recording backup unavailable: {error}"
                    ))
                    .to_user_error(),
                );
                return Err(error.into());
            }
        };
        let stt_control = SttTaskControl {
            id: session_id,
            done: Arc::new(Notify::new()),
            abort: Arc::new(Notify::new()),
            remote_polish: Arc::new(Mutex::new(remote_polish_options(&config_data))),
            raw_transcript: Arc::new(Mutex::new(None)),
            journal,
        };
        *self.stt_session.lock().unwrap_or_else(|e| e.into_inner()) = Some(stt_control.clone());
        let session_ref = self.session.clone();
        let stt_error_ref = self.stt_error.clone();
        let live_session = options.live_typing;
        let live_idle_stop_seconds = config_data.live_idle_stop_seconds;
        let live_sender = if options.live_typing {
            self.spawn_live_insert_worker(&config_data)
        } else {
            None
        };

        tokio::spawn(async move {
            // Forward audio to STT and receive transcripts
            loop {
                if !session_ref.is_live(stt_control.id) {
                    break;
                }

                tokio::select! {
                    _ = stt_control.abort.notified() => {
                        tracing::info!("STT task received abort signal");
                        break;
                    }
                    chunk = audio_rx.recv() => {
                        match chunk {
                            Some(data) => {
                                let backed_up = stt_control.journal.lock().unwrap_or_else(|error| error.into_inner()).append(&data);
                                let sent = match backed_up {
                                    Ok(()) => provider.send_audio(&data).await,
                                    Err(error) => Err(crate::error::AppError::Config(format!("Recording backup failed: {error}"))),
                                };
                                if let Err(error) = sent {
                                    tracing::error!("STT send audio error: {}", error);
                                    crate::error::emit_cloud_session_invalid(&app_handle, &error);
                                    if let Some(user_error) = latch_stt_task_error_if_active(
                                        &session_ref,
                                        stt_error_ref.as_ref(),
                                        stt_control.id,
                                        &error,
                                    ) {
                                        let _ = app_handle.emit("pipeline:error", user_error);
                                    }
                                    break;
                                }
                            }
                            None => {
                                // Audio channel closed — disconnect and capture final transcript
                                if !session_ref.is_live(stt_control.id) {
                                    break;
                                }

                                let options = stt_control.remote_polish.lock().unwrap_or_else(|error| error.into_inner()).clone();
                                if let Some(options) = options {
                                    provider.set_remote_polish(&options);
                                }
                                if let Err(error) = stt_control.journal.lock().unwrap_or_else(|error| error.into_inner()).finish_audio() {
                                    tracing::error!("Recording checkpoint failed: {error}");
                                    let error = crate::error::AppError::Config(format!("Recording backup failed: {error}"));
                                    if let Some(user_error) = latch_stt_task_error_if_active(
                                        &session_ref,
                                        stt_error_ref.as_ref(),
                                        stt_control.id,
                                        &error,
                                    ) {
                                        let _ = app_handle.emit("pipeline:error", user_error);
                                    }
                                    break;
                                }
                                let disconnect_result = tokio::select! {
                                    _ = stt_control.abort.notified() => None,
                                    result = provider.disconnect() => Some(result),
                                };

                                match disconnect_result {
                                    Some(Ok(Some(text))) => {
                                        if session_ref.is_live(stt_control.id) {
                                            *stt_control.raw_transcript.lock().unwrap_or_else(|error| error.into_inner()) = provider.raw_transcript();
                                            let original = provider.raw_transcript().unwrap_or_else(|| text.clone());
                                            if let Err(error) = stt_control.journal.lock().unwrap_or_else(|error| error.into_inner()).save_transcript(&original) {
                                                tracing::error!("Transcript checkpoint failed; original audio remains: {error}");
                                            }
                                            let mut acc = accumulated.lock().unwrap_or_else(|e| e.into_inner());
                                            acc.push_str(&text);
                                            let current = acc.clone();
                                            drop(acc);
                                            let _ = app_handle.emit("stt:final", &current);
                                        }
                                    }
                                    Some(Ok(None)) => {}
                                    Some(Err(e)) => {
                                        tracing::error!("STT disconnect error: {}", e);
                                        if session_ref.is_live(stt_control.id) {
                                            crate::error::emit_cloud_session_invalid(
                                                &app_handle,
                                                &e,
                                            );
                                            let user_error = e.to_user_error();
                                            *stt_error_ref.lock().unwrap_or_else(|e| e.into_inner()) =
                                                Some((stt_control.id, user_error.clone()));
                                            let _ = app_handle.emit("pipeline:error", user_error);
                                        }
                                    }
                                    None => {}
                                }
                                break;
                            }
                        }
                    }
                    transcript = provider.recv_transcript() => {
                        match transcript {
                            Ok(Some(TranscriptEvent::Partial { text })) => {
                                if session_ref.is_live(stt_control.id) {
                                    let _ = app_handle.emit("stt:partial", &text);
                                }
                            }
                            Ok(Some(TranscriptEvent::SpeechEnded)) if live_session => {
                                // Live typing heard nothing for a while: end the session
                                // like a second press of the shortcut.
                                if session_ref.is_live(stt_control.id) {
                                    tracing::info!("Live typing stopped after silence");
                                    let _ = app_handle.emit(
                                        "pipeline:live_idle_stop",
                                        live_idle_stop_seconds,
                                    );
                                    let handle = app_handle.clone();
                                    tauri::async_runtime::spawn(async move {
                                        let pipeline = handle.state::<PipelineHandle>();
                                        if let Err(error) = pipeline.stop().await {
                                            tracing::warn!("Live typing idle stop failed: {error}");
                                        }
                                    });
                                }
                            }
                            Ok(Some(TranscriptEvent::Committed { text })) => {
                                // The provider's final transcript repeats this text; stop()
                                // types whatever the worker has not inserted yet.
                                if let Some(sender) = live_sender.as_ref() {
                                    if session_ref.is_live(stt_control.id) {
                                        let _ = sender.send(text);
                                    }
                                }
                            }
                            Ok(Some(TranscriptEvent::Final { text, .. })) => {
                                if session_ref.is_live(stt_control.id) {
                                    let mut acc = accumulated.lock().unwrap_or_else(|e| e.into_inner());
                                    acc.push_str(&text);
                                    acc.push(' ');
                                    let current = acc.clone();
                                    drop(acc);
                                    let _ = app_handle.emit("stt:final", &current);
                                }
                            }
                            Ok(Some(TranscriptEvent::Error { message })) => {
                                tracing::error!("STT error: {}", message);
                                if session_ref.is_live(stt_control.id) {
                                    let user_error =
                                        crate::error::AppError::Config(message.clone()).to_user_error();
                                    *stt_error_ref.lock().unwrap_or_else(|e| e.into_inner()) =
                                        Some((stt_control.id, user_error.clone()));
                                    let _ = app_handle.emit("pipeline:error", user_error);
                                }
                                // Break out of the loop — STT has failed, no point
                                // continuing. Without break, the loop keeps running
                                // and the pipeline stays stuck in Recording forever.
                                break;
                            }
                            Err(e) => {
                                tracing::error!("STT recv error: {}", e);
                                if session_ref.is_live(stt_control.id) {
                                    crate::error::emit_cloud_session_invalid(&app_handle, &e);
                                    let user_error = e.to_user_error();
                                    *stt_error_ref.lock().unwrap_or_else(|e| e.into_inner()) =
                                        Some((stt_control.id, user_error.clone()));
                                    let _ = app_handle.emit("pipeline:error", user_error);
                                }
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Signal that STT processing is complete
            stt_control.done.notify_one();
        });

        let deadline_pipeline = self.clone();
        let deadline_app = self.app_handle.clone();
        let deadline_scope = self.session.recording_scope(session_id, &self.phases);
        let deadline_provider_id = resolved_limit.capability.provider_id;
        tokio::spawn(async move {
            let reached = crate::recording_deadline::drive_recording_deadline(
                recording_deadline,
                || deadline_scope.is_active(),
                |signal| match signal {
                    crate::recording_deadline::RecordingDeadlineSignal::Warning {
                        seconds_remaining,
                    } => {
                        let _ = deadline_app.emit(
                            "recording:deadline-warning",
                            serde_json::json!({
                                "sessionId": session_id,
                                "recordingKind": "dictation",
                                "secondsRemaining": seconds_remaining,
                                "effectiveMaxSeconds": effective_max_seconds,
                                "providerId": deadline_provider_id.as_str(),
                                "explanationKey": deadline_explanation_key.as_str(),
                            }),
                        );
                    }
                    crate::recording_deadline::RecordingDeadlineSignal::Reached => {
                        let _ = deadline_app.emit(
                            "recording:deadline-reached",
                            serde_json::json!({
                                "sessionId": session_id,
                                "recordingKind": "dictation",
                                "effectiveMaxSeconds": effective_max_seconds,
                                "providerId": deadline_provider_id.as_str(),
                                "explanationKey": deadline_explanation_key.as_str(),
                            }),
                        );
                    }
                },
            )
            .await;
            if reached {
                if let Err(error) = deadline_pipeline.stop().await {
                    tracing::error!("Failed to stop recording at the provider deadline: {error}");
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        // Acquire pipeline_lock so we wait for start() to finish its setup
        // (load config, initialize audio and connect STT) before reading shared state.
        // Released before the long stt_done wait so start() isn't blocked 120s.
        let guard = self.pipeline_lock.lock().await;
        let generation = self.session.current();
        let mut cancelled = self.session.watch();

        // Atomic CAS: only one caller can transition Recording → Transcribing
        if !self
            .phases
            .advance(PipelineState::Recording, PipelineState::Transcribing)
        {
            return Ok(());
        }
        let current_config = self.load_config().await;
        let finalized_translation_target = self
            .active_translation_operation
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_mut()
            .map(TranslationOperationState::finalize);
        let stop_start = std::time::Instant::now();

        let original_target = self
            .preloaded_app_ctx
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .map(|context| context.target_guard.clone());
        if let Some(target) = original_target {
            self.restore_after_capsule_interaction(&target).await;
        }

        // Capture selected text now — hotkey is released so Ctrl+C won't conflict.
        // Small delay to ensure hotkey modifiers are fully released (especially in toggle mode).
        let config_data = self
            .preloaded_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap_or_default();
        let live_typing = self.live_typing.load(Ordering::SeqCst);
        // Live typing already wrote into the target; a Ctrl+C capture would copy that text.
        let selected_text = if config_data.selected_text_enabled && !live_typing {
            tokio::time::sleep(std::time::Duration::from_millis(
                SELECTED_TEXT_CAPTURE_DELAY_MS,
            ))
            .await;
            tokio::task::block_in_place(|| self.capture_selected_text())
        } else {
            None
        };
        tracing::info!(
            "Selected text result: len={}",
            selected_text.as_deref().map(|s| s.len()).unwrap_or(0)
        );
        *self
            .preloaded_selected_text
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = selected_text;

        let stt_control = self
            .stt_session
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if let Some(control) = &stt_control {
            *control
                .remote_polish
                .lock()
                .unwrap_or_else(|error| error.into_inner()) =
                remote_polish_options(&current_config);
        }

        // Stop audio capture (this drops the channel, signaling STT task to stop)
        {
            let mut handle = self.audio_handle.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref mut h) = *handle {
                if let (Some(message), Some(control)) = (h.runtime_error(), stt_control.as_ref()) {
                    let error = crate::error::AppError::Config(format!(
                        "Audio capture interrupted: {message}"
                    ));
                    if let Some(user_error) = latch_stt_task_error_if_active(
                        &self.session,
                        self.stt_error.as_ref(),
                        control.id,
                        &error,
                    ) {
                        let _ = self.app_handle.emit("pipeline:error", user_error);
                    }
                }
                h.stop();
            }
            *handle = None;
        }

        // P2-1: Pre-build LLM resources while waiting for STT
        let preloaded_config = self
            .preloaded_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let mut config = match preloaded_config {
            Some(c) => c,
            None => self.load_config().await,
        };
        config.polish_style = current_config.polish_style;
        config.polish_enabled = current_config.polish_enabled;
        if let Some(target) = finalized_translation_target {
            config.translation.active_target = target.clone();
            config.target_lang = target;
        }
        let app_ctx = self
            .preloaded_app_ctx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .unwrap_or_else(|| {
                self.context_detector
                    .snapshot_for_recording_enabled(config.context_adaptation_enabled)
            });
        let dictionary_words = self
            .preloaded_dictionary
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .unwrap_or_default();
        let correction_rules = self
            .preloaded_correction_rules
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .unwrap_or_default();
        let selected_text = self
            .preloaded_selected_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let operation_id = self
            .cloud_operation_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let voice_mode = self
            .preloaded_voice_mode
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .unwrap_or(crate::voice_intent::VoiceMode::Dictate);

        // Extract session token before releasing guard (for cloud LLM)
        let session_token = if config.llm_provider == "cloud" {
            self.app_handle
                .state::<SessionTokenStore>()
                .0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            String::new()
        };

        // All shared state has been taken — release the lock so a new start()
        // isn't blocked by the long stt_done wait that follows.
        drop(guard);

        // ── Phase 1: Wait for STT ──────────────────────────────────────
        let raw_text = match tokio::select! {
            biased;
            _ = cancelled.wait_for(|value| *value != generation) => return Ok(()),
            result = self.wait_for_stt(stt_control.clone(), generation) => result?,
        } {
            Some(text) => text,
            None => {
                if let Some(control) = &stt_control {
                    self.clear_stt_session(control.id);
                }
                return Ok(());
            } // aborted or no speech detected
        };
        let voice_intent =
            route_pipeline_voice_intent(voice_mode, &raw_text, selected_text.as_deref(), &config);
        if let Some(control) = &stt_control {
            let original = control
                .raw_transcript
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone()
                .unwrap_or_else(|| raw_text.clone());
            if let Err(error) = control
                .journal
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .save_transcript(&original)
            {
                tracing::error!("Transcript checkpoint failed; original audio remains: {error}");
            }
        }
        let stt_elapsed = stop_start.elapsed();
        tracing::info!(
            "[Pipeline Timing] STT finalize: {}ms",
            stt_elapsed.as_millis()
        );

        // Check abort before entering LLM polish and output
        if !self.session.is_live(generation) {
            tracing::info!("Pipeline aborted before LLM/output");
            if let Some(control) = &stt_control {
                self.clear_stt_session(control.id);
            }
            return Ok(());
        }

        // Synthesised key events are dropped by macOS without the Accessibility
        // grant, with no error anywhere. Say so plainly instead of failing silently.
        #[cfg(target_os = "macos")]
        if !is_accessibility_trusted()
            && matches!(
                config.insertion_strategy.as_str(),
                "auto" | "keyboard" | "windowsSendInput"
            )
        {
            tracing::warn!(
                "macOS Accessibility permission is not granted; enable Ultra STT in System Settings > Privacy & Security > Accessibility to type into the focused app."
            );
        }
        // ── Phase 2: LLM polish + output ───────────────────────────────
        let polish_outcome = if live_typing {
            tokio::select! {
                biased;
                _ = cancelled.wait_for(|value| *value != generation) => return Ok(()),
                outcome = self.finish_live_typing(&raw_text, &app_ctx, &config, generation) => outcome,
            }
        } else {
            tokio::select! {
            biased;
            _ = cancelled.wait_for(|value| *value != generation) => return Ok(()),
            outcome = self.polish_text(PolishTextInput {
                raw_text: &raw_text,
                voice_mode,
                config: &config,
                app_ctx: &app_ctx,
                dictionary_words,
                correction_rules,
                selected_text,
                session_token,
                operation_id,
                voice_intent,
                popup_fallback_enabled: true,
                session_generation: generation,
            }) => outcome,
            }
        };
        if !self.session.is_live(generation) {
            return Ok(());
        }
        let final_text = polish_outcome.final_text;
        let llm_elapsed = polish_outcome.llm_elapsed;

        // ── Phase 3: Timing, history, cleanup ──────────────────────────
        let total_elapsed = stop_start.elapsed();

        // Compute recording duration
        let duration_ms = self
            .recording_start
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .map(|start| start.elapsed().as_millis() as i64);

        tracing::info!(
            "[Pipeline Timing] Total stop(): {}ms (STT: {}ms, LLM: {}ms, Output+Save: {}ms)",
            total_elapsed.as_millis(),
            stt_elapsed.as_millis(),
            llm_elapsed.as_millis(),
            total_elapsed.as_millis() - stt_elapsed.as_millis() - llm_elapsed.as_millis(),
        );

        // A new recording may have superseded this stop during output.
        if !self.session.is_live(generation) {
            return Ok(());
        }
        // Emit timing to frontend
        let _ = self.app_handle.emit(
            "pipeline:timing",
            serde_json::json!({
                "stt_ms": stt_elapsed.as_millis() as u64,
                "llm_ms": llm_elapsed.as_millis() as u64,
                "total_ms": total_elapsed.as_millis() as u64,
                "recording_ms": duration_ms,
            }),
        );

        let _ = self.app_handle.emit("pipeline:context", app_ctx.summary());

        // Save to history
        if !self.session.is_live(generation) {
            return Ok(());
        }
        let history_raw = stt_control
            .as_ref()
            .and_then(|control| {
                control
                    .raw_transcript
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .clone()
            })
            .unwrap_or_else(|| raw_text.clone());
        let confirmed_output = matches!(
            polish_outcome.history_output_status.as_deref(),
            Some("inserted" | "copied")
        );
        self.save_history(
            &history_raw,
            &final_text,
            &app_ctx,
            duration_ms,
            &config,
            HistoryOutputMetadata {
                status: polish_outcome.history_output_status,
                error: polish_outcome.history_output_error,
            },
        )
        .await;

        if let Some(control) = &stt_control {
            if confirmed_output {
                if let Err(error) = control
                    .journal
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .discard()
                {
                    tracing::error!("Could not clean completed recording backup: {error}");
                }
            }
            self.clear_stt_session(control.id);
        }
        if self.session.is_live(generation) {
            self.set_state(PipelineState::Idle);
        }
        Ok(())
    }

    /// Wait for the STT task to complete and return the transcribed text.
    /// Returns `Ok(Some(text))` on success, `Ok(None)` if aborted or no speech,
    /// or `Err` on failure.
    async fn wait_for_stt(
        &self,
        stt_control: Option<SttTaskControl>,
        generation: u64,
    ) -> Result<Option<String>> {
        if let Some(control) = &stt_control {
            let recorded_seconds = self
                .recording_start
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .as_ref()
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(600);
            let budget = crate::stt::processing_budget::finalize(recorded_seconds);
            tokio::select! {
                _ = control.done.notified() => {
                    tracing::debug!("STT task completed");
                }
                _ = tokio::time::sleep(budget) => {
                    control.abort.notify_one();
                    tracing::warn!("STT finalization timed out after {}s", budget.as_secs());
                    if self.session.is_live(generation) {
                        let _ = self.app_handle.emit("pipeline:error", crate::error::AppError::Timeout(budget).to_user_error());
                        self.set_state(PipelineState::Idle);
                    }
                    return Ok(None);
                }
            }

            if !self.session.is_live(control.id) {
                tracing::info!("Ignoring stale or aborted STT task");
                return Ok(None);
            }
            if take_matching_stt_error(&self.stt_error, control.id).is_some() {
                if self.session.is_live(generation) {
                    self.set_state(PipelineState::Idle);
                }
                return Ok(None);
            }
        } else {
            tracing::warn!("No STT session was available to wait for");
        }

        if !self.session.is_live(generation) {
            tracing::info!("Pipeline aborted after STT wait");
            return Ok(None);
        }

        let raw_text = self
            .accumulated_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .trim()
            .to_string();

        if raw_text.is_empty() {
            if self.session.is_live(generation) {
                let _ = self
                    .app_handle
                    .emit("pipeline:error", no_speech_user_error());
                self.set_state(PipelineState::Idle);
            }
            return Ok(None);
        }

        Ok(Some(raw_text))
    }

    /// Polish raw text with LLM and output the result.
    /// Returns (final_text, llm_elapsed_duration).
    async fn polish_text(&self, input: PolishTextInput<'_>) -> PolishTextOutcome {
        let PolishTextInput {
            raw_text,
            voice_mode,
            config,
            app_ctx,
            dictionary_words,
            correction_rules,
            selected_text,
            session_token,
            operation_id,
            voice_intent,
            popup_fallback_enabled,
            session_generation,
        } = input;
        if remote_output_is_final(
            config,
            voice_mode,
            voice_intent.kind,
            selected_text.as_deref(),
        ) {
            let output_result = self
                .output_text(
                    raw_text,
                    &app_ctx.profile.app_label,
                    &app_ctx.target_guard,
                    config,
                    session_generation,
                )
                .await;
            if let Err(error) = &output_result {
                tracing::error!("Output failed: {}", error);
                let _ = self
                    .app_handle
                    .emit("pipeline:error", output_user_error(error));
            }
            let output_metadata = history_output_metadata_for_output_result(
                output_result.as_ref(),
                output::InsertionStrategy::from_config_value(&config.insertion_strategy)
                    == output::InsertionStrategy::ClipboardCopyOnly,
            );
            return PolishTextOutcome {
                final_text: raw_text.to_string(),
                llm_elapsed: std::time::Duration::ZERO,
                history_output_status: output_metadata.status,
                history_output_error: output_metadata.error,
                voice_execution: None,
            };
        }
        let provider_plan =
            crate::voice_intent::plan_voice_provider_work(voice_mode, raw_text, &voice_intent);
        let Some(provider_text) = provider_plan.provider_input.as_deref() else {
            let message = "Search must bypass the language model and use the safe search executor.";
            let _ = self.app_handle.emit(
                "pipeline:error",
                crate::error::UserError {
                    code: "voice_route_failed".to_string(),
                    details: Some(message.to_string()),
                    retry_count: 0,
                },
            );
            return PolishTextOutcome::with_history_status(
                String::new(),
                std::time::Duration::ZERO,
                "fallback",
                message,
            );
        };
        let llm_api_key = if config.llm_provider == "cloud" {
            session_token
        } else if config.llm_provider == "codex-cli" {
            String::new()
        } else {
            match resolve_llm_config_secret(config, &SystemCredentialVault) {
                Ok(secret) => secret,
                Err(error) => {
                    tracing::warn!("Failed to read LLM credential: {error}");
                    String::new()
                }
            }
        };

        // Check if polish is enabled and API key / token is available
        if !config.polish_enabled
            || (config.llm_provider != "cloud"
                && !llm::has_usable_provider_credentials(&config.llm_provider, &llm_api_key))
        {
            if selected_text_command_requires_llm(selected_text.as_deref())
                || voice_intent_requires_generated_output(voice_intent.kind)
            {
                let message = if !config.polish_enabled {
                    "This voice command needs AI polish. Enable AI polish before drafting, translating, or editing."
                } else {
                    "This voice command needs a configured LLM provider or Cloud sign-in."
                };
                if let Err(error) =
                    crate::commands::ask::show_error_window(&self.app_handle, message.to_string())
                {
                    tracing::warn!("Failed to show selected-text no-LLM Ask error window: {error}");
                    let _ = self.app_handle.emit(
                        "pipeline:error",
                        crate::error::UserError {
                            code: "llm_failed".to_string(),
                            details: Some(message.to_string()),
                            retry_count: 0,
                        },
                    );
                }
                return PolishTextOutcome::with_history_status(
                    String::new(),
                    std::time::Duration::ZERO,
                    "fallback",
                    message,
                );
            }

            // No polishing — output raw text directly
            let output_result = self
                .output_text(
                    provider_text,
                    &app_ctx.profile.app_label,
                    &app_ctx.target_guard,
                    config,
                    session_generation,
                )
                .await;
            if let Err(error) = &output_result {
                tracing::error!("Output failed: {}", error);
                let _ = self
                    .app_handle
                    .emit("pipeline:error", output_user_error(error));
            }
            let output_metadata = history_output_metadata_for_output_result(
                output_result.as_ref(),
                output::InsertionStrategy::from_config_value(&config.insertion_strategy)
                    == output::InsertionStrategy::ClipboardCopyOnly,
            );
            return PolishTextOutcome {
                final_text: provider_text.to_string(),
                llm_elapsed: std::time::Duration::ZERO,
                history_output_status: output_metadata.status,
                history_output_error: output_metadata.error,
                voice_execution: None,
            };
        }

        self.set_state(PipelineState::Polishing);
        let llm_start = std::time::Instant::now();

        let llm_config = LlmConfig {
            provider: config.llm_provider.clone(),
            api_key: llm_api_key,
            model: config.llm_model.clone(),
            base_url: config.llm_base_url.clone(),
            max_tokens: 4096,
            temperature: 0.3,
        };
        let provider = llm::create_provider(&config.llm_provider, Some(self.shared_client.clone()));

        let streaming_strategy = (config.llm_provider != "codex-cli"
            && provider_plan.allow_streaming)
            .then(|| streaming_insert_strategy_for_runtime(config, selected_text.as_deref()))
            .flatten();
        let mut streaming_worker = streaming_strategy.map(|strategy| {
            spawn_streaming_insert_worker(StreamingInsertWorkerContext {
                app_handle: self.app_handle.clone(),
                session: self.session.clone(),
                session_id: session_generation,
                context_detector: self.context_detector.clone(),
                strategy,
                windows_sendinput_options: output::windows_sendinput::WindowsSendInputOptions {
                    newline_mode:
                        output::windows_sendinput::WindowsSendInputNewlineMode::from_config_value(
                            &config.windows_sendinput_newline_mode,
                        ),
                },
                expected_target_guard: app_ctx.target_guard.clone(),
                expected_target_label: app_ctx.profile.app_label.clone(),
            })
        });
        let streaming_sender = streaming_worker
            .as_ref()
            .map(|worker| worker.sender.clone());

        // The callback remains synchronous for the LLM stream. UI updates happen
        // immediately; optional target-app insertion is drained by a worker.
        let app_handle = self.app_handle.clone();
        let chunk_session = self.session.clone();
        let on_chunk: llm::ChunkCallback = Box::new(move |chunk: &str| {
            if !chunk_session.is_live(session_generation) {
                return;
            }
            let _ = app_handle.emit("llm:chunk", chunk);
            if let Some(sender) = streaming_sender.as_ref() {
                let _ = sender.send(chunk.to_string());
            }
        });

        let selected_text_for_execution = selected_text.clone();
        let mapped_scene_prompt = storage::automatic_scene_prompt(
            config,
            app_ctx.profile.family,
            app_ctx.mapped_scene_id.as_deref(),
        )
        .unwrap_or_default();
        let req = PolishRequest {
            raw_text: provider_text.to_string(),
            context: app_ctx.summary(),
            dictionary: dictionary_words,
            correction_rules,
            polish_style: config.polish_style.clone(),
            mapped_scene_prompt,
            active_scene_prompt: config
                .active_scene
                .as_ref()
                .map(|scene| scene.prompt_template.clone())
                .unwrap_or_default(),
            polish_custom_prompt: config.polish_custom_prompt.clone(),
            translate_enabled: config.translate_enabled,
            target_lang: config.translation.active_target.clone(),
            selected_text,
            operation_id,
            voice_intent: voice_intent.clone(),
        };

        let polish_result = provider
            .polish(
                &llm_config,
                &req,
                (config.llm_provider != "codex-cli").then_some(&on_chunk),
            )
            .await;
        drop(on_chunk);
        let streaming_report = match streaming_worker.take() {
            Some(worker) => worker.finish().await,
            None => None,
        };
        if !self.session.is_live(session_generation) {
            return PolishTextOutcome::normal(String::new(), llm_start.elapsed());
        }

        let polish_outcome = match polish_result {
            Ok(response) => {
                let elapsed = llm_start.elapsed();
                if let Some(report) = streaming_report.as_ref() {
                    if report.has_inserted_text() {
                        let mut streaming_history_status: Option<(&'static str, String)> = None;
                        match streaming_recovery_action(
                            report,
                            Some(&response.polished_text),
                            true,
                            !report.failed && !report.target_lost,
                        ) {
                            StreamingRecoveryAction::AlreadyComplete => {
                                self.emit_streaming_insert_result(
                                    report,
                                    &app_ctx.profile.app_label,
                                );
                            }
                            StreamingRecoveryAction::InsertSuffix { suffix } => {
                                let mut recovered_report = report.clone();
                                if !suffix.is_empty() {
                                    if !self
                                        .context_detector
                                        .target_still_matches_now(&app_ctx.target_guard)
                                    {
                                        recovered_report.target_lost = true;
                                        self.copy_streaming_recovery_to_clipboard(
                                            &response.polished_text,
                                            "Target changed before the remaining text could be inserted".to_string(),
                                            &app_ctx.profile.app_label,
                                            config,
                                            session_generation,
                                            &recovered_report,
                                        ).await;
                                        return PolishTextOutcome::with_history_status(
                                            response.polished_text,
                                            elapsed,
                                            "clipboard_fallback",
                                            "Target changed before final streaming suffix",
                                        );
                                    }
                                    match output::output_stream_chunk(
                                        &self.app_handle,
                                        &suffix,
                                        report.strategy,
                                        output::windows_sendinput::WindowsSendInputOptions {
                                            newline_mode: output::windows_sendinput::WindowsSendInputNewlineMode::from_config_value(
                                                &config.windows_sendinput_newline_mode,
                                            ),
                                        },
                                    )
                                    .await
                                    {
                                        Ok(insert_result)
                                            if insert_result.status == output::InsertStatus::Inserted
                                                && insert_result.chars_inserted
                                                    == suffix.chars().count() =>
                                        {
                                            recovered_report.inserted_text =
                                                response.polished_text.clone();
                                            recovered_report.failed = false;
                                            recovered_report.error_message = None;
                                            recovered_report.attempted_chunks =
                                                recovered_report.attempted_chunks.saturating_add(1);
                                            self.emit_streaming_insert_result(
                                                &recovered_report,
                                                &app_ctx.profile.app_label,
                                            );
                                        }
                                        Ok(insert_result) => {
                                            recovered_report.inserted_text.extend(
                                                suffix.chars().take(insert_result.chars_inserted),
                                            );
                                            let reason = format!(
                                                "streaming suffix insert stopped with status {:?}",
                                                insert_result.status
                                            );
                                            recovered_report.failed = true;
                                            recovered_report.error_message = Some(reason.clone());
                                            streaming_history_status =
                                                Some(("clipboard_fallback", reason.clone()));
                                            self.emit_streaming_insert_result(
                                                &recovered_report,
                                                &app_ctx.profile.app_label,
                                            );
                                            self.copy_streaming_recovery_to_clipboard(
                                                &response.polished_text,
                                                reason,
                                                &app_ctx.profile.app_label,
                                                config,
                                                session_generation,
                                                &recovered_report,
                                            )
                                            .await;
                                        }
                                        Err(error) => {
                                            recovered_report.failed = true;
                                            recovered_report.error_message = Some(error.clone());
                                            streaming_history_status =
                                                Some(("clipboard_fallback", error.clone()));
                                            self.emit_streaming_insert_result(
                                                &recovered_report,
                                                &app_ctx.profile.app_label,
                                            );
                                            self.copy_streaming_recovery_to_clipboard(
                                                &response.polished_text,
                                                error,
                                                &app_ctx.profile.app_label,
                                                config,
                                                session_generation,
                                                &recovered_report,
                                            )
                                            .await;
                                        }
                                    }
                                } else {
                                    self.emit_streaming_insert_result(
                                        &recovered_report,
                                        &app_ctx.profile.app_label,
                                    );
                                }
                            }
                            StreamingRecoveryAction::CopyFullToClipboard { reason } => {
                                let mut partial_report = report.clone();
                                partial_report.failed = true;
                                partial_report.error_message = Some(reason.clone());
                                streaming_history_status =
                                    Some(("clipboard_fallback", reason.clone()));
                                self.emit_streaming_insert_result(
                                    &partial_report,
                                    &app_ctx.profile.app_label,
                                );
                                self.copy_streaming_recovery_to_clipboard(
                                    &response.polished_text,
                                    reason,
                                    &app_ctx.profile.app_label,
                                    config,
                                    session_generation,
                                    report,
                                )
                                .await;
                            }
                            StreamingRecoveryAction::CopyPartialToClipboard { reason } => {
                                let mut partial_report = report.clone();
                                partial_report.failed = true;
                                partial_report.error_message = Some(reason.clone());
                                streaming_history_status = Some(("partial", reason.clone()));
                                self.emit_streaming_insert_result(
                                    &partial_report,
                                    &app_ctx.profile.app_label,
                                );
                                self.copy_streaming_recovery_to_clipboard(
                                    &partial_report.inserted_text,
                                    reason,
                                    &app_ctx.profile.app_label,
                                    config,
                                    session_generation,
                                    report,
                                )
                                .await;
                            }
                            StreamingRecoveryAction::NoRecoveryNeeded => {}
                        }
                        if let Some((status, error)) = streaming_history_status {
                            return PolishTextOutcome::with_history_status(
                                response.polished_text,
                                elapsed,
                                status,
                                error,
                            );
                        }
                        return PolishTextOutcome::normal(response.polished_text, elapsed);
                    }
                    if report.target_lost {
                        let reason = report.error_message.clone().unwrap_or_else(|| {
                            "target app changed before streaming insert".to_string()
                        });
                        self.copy_streaming_recovery_to_clipboard(
                            &response.polished_text,
                            reason,
                            &app_ctx.profile.app_label,
                            config,
                            session_generation,
                            report,
                        )
                        .await;
                        return PolishTextOutcome::with_history_status(
                            response.polished_text,
                            elapsed,
                            "clipboard_fallback",
                            "Target app changed before streaming insert; copied full result to clipboard",
                        );
                    }
                    if report.failed {
                        tracing::warn!(
                            "Streaming insert failed before typing; falling back to one-shot output"
                        );
                    }
                }

                // Check abort after LLM returns — skip output if cancelled during polish.
                if !self.session.is_live(session_generation) {
                    tracing::info!("Pipeline aborted after LLM polish, skipping output");
                    return PolishTextOutcome::normal(raw_text.to_string(), elapsed);
                }

                let selected_text_available = if voice_intent.placement
                    == crate::voice_intent::VoiceOutputPlacement::ReplaceSelection
                {
                    selected_text_for_execution
                        .as_deref()
                        .is_some_and(|original| {
                            tokio::task::block_in_place(crate::selection::capture_selected_text)
                                .is_some_and(|current| current.trim() == original.trim())
                        })
                } else {
                    selected_text_has_content(selected_text_for_execution.as_deref())
                };
                let mut backend = PipelineVoiceExecutionBackend {
                    pipeline: self,
                    app_name: &app_ctx.profile.app_label,
                    question: raw_text,
                    intent_kind: voice_intent.kind,
                    target_guard: &app_ctx.target_guard,
                    config,
                    already_copied: false,
                    popup_fallback_enabled,
                    session_generation,
                };
                let execution = crate::voice_intent::executor::execute_voice_intent(
                    crate::voice_intent::executor::VoiceExecutionRequest {
                        intent: &voice_intent,
                        generated_output: &response.polished_text,
                        target_guard: &app_ctx.target_guard,
                        selected_text_available,
                        restore_target_before_insert: provider_plan.restore_target_before_insert,
                        flags: config.voice_routing_flags,
                    },
                    &mut backend,
                )
                .await;
                let _ = self.app_handle.emit("pipeline:voice_execution", &execution);

                if let Some(reason) = crate::recovery::reason_for_voice_fallback(
                    execution.fallback_reason,
                    output::InsertionStrategy::from_config_value(&config.insertion_strategy)
                        == output::InsertionStrategy::ClipboardCopyOnly,
                    backend.already_copied,
                ) {
                    self.retain_recovery(
                        &response.polished_text,
                        &app_ctx.profile.app_label,
                        session_generation,
                        reason,
                        0,
                    );
                }

                let (history_status, history_error) = match execution.status {
                    crate::voice_intent::executor::VoiceExecutionStatus::Completed => (None, None),
                    crate::voice_intent::executor::VoiceExecutionStatus::CopiedFallback => (
                        Some("clipboard_fallback".to_string()),
                        Some(format!(
                            "Voice output fallback: {:?}",
                            execution.fallback_reason
                        )),
                    ),
                    crate::voice_intent::executor::VoiceExecutionStatus::PopupFallback
                    | crate::voice_intent::executor::VoiceExecutionStatus::Prevented
                    | crate::voice_intent::executor::VoiceExecutionStatus::Failed => (
                        Some("fallback".to_string()),
                        Some(format!(
                            "Voice output fallback: {:?}",
                            execution.fallback_reason
                        )),
                    ),
                };
                PolishTextOutcome::with_execution(
                    response.polished_text,
                    elapsed,
                    execution,
                    history_status,
                    history_error,
                )
            }
            Err(e) => {
                crate::error::emit_cloud_session_invalid(&self.app_handle, &e);
                let elapsed = llm_start.elapsed();
                if let Some(report) = streaming_report.as_ref() {
                    if report.has_inserted_text() {
                        tracing::error!("LLM polish failed after partial streaming insert: {}", e);
                        let _ = self
                            .app_handle
                            .emit("pipeline:error", llm_polish_user_error(&e));
                        let mut partial_report = report.clone();
                        partial_report.failed = true;
                        partial_report.error_message = Some(format!("LLM polish failed: {}", e));
                        self.emit_streaming_insert_result(
                            &partial_report,
                            &app_ctx.profile.app_label,
                        );
                        if let StreamingRecoveryAction::CopyPartialToClipboard { reason } =
                            streaming_recovery_action(&partial_report, None, false, false)
                        {
                            self.copy_streaming_recovery_to_clipboard(
                                &partial_report.inserted_text,
                                reason,
                                &app_ctx.profile.app_label,
                                config,
                                session_generation,
                                report,
                            )
                            .await;
                        }
                        return PolishTextOutcome::with_history_status(
                            partial_report.inserted_text,
                            elapsed,
                            "partial",
                            format!("LLM polish failed after partial streaming insert: {e}"),
                        );
                    }
                }

                // Check abort after LLM error — skip fallback output if cancelled.
                if !self.session.is_live(session_generation) {
                    tracing::info!("Pipeline aborted after LLM error, skipping output");
                    return PolishTextOutcome::normal(String::new(), elapsed);
                }
                tracing::error!("LLM polish failed: {}", e);

                let _ = self
                    .app_handle
                    .emit("pipeline:error", llm_polish_user_error(&e));
                if config.llm_provider == "codex-cli"
                    || selected_text_has_content(selected_text_for_execution.as_deref())
                    || voice_intent_requires_generated_output(voice_intent.kind)
                {
                    return PolishTextOutcome::with_history_status(
                        String::new(),
                        elapsed,
                        "fallback",
                        "LLM generation failed; no application text was changed",
                    );
                }
                let output_result = self
                    .output_text(
                        provider_text,
                        &app_ctx.profile.app_label,
                        &app_ctx.target_guard,
                        config,
                        session_generation,
                    )
                    .await;
                if let Err(output_error) = &output_result {
                    tracing::error!("Output failed: {}", output_error);
                    let _ = self
                        .app_handle
                        .emit("pipeline:error", output_user_error(output_error));
                }
                let output_metadata = history_output_metadata_with_prior_error(
                    output_result.as_ref(),
                    output::InsertionStrategy::from_config_value(&config.insertion_strategy)
                        == output::InsertionStrategy::ClipboardCopyOnly,
                    &format!("LLM polish failed: {e}"),
                );
                PolishTextOutcome {
                    final_text: provider_text.to_string(),
                    llm_elapsed: elapsed,
                    history_output_status: output_metadata.status,
                    history_output_error: output_metadata.error,
                    voice_execution: None,
                }
            }
        };

        tracing::info!(
            "[Pipeline Timing] LLM polish: {}ms",
            polish_outcome.llm_elapsed.as_millis()
        );

        polish_outcome
    }

    pub(crate) async fn run_ask_draft(
        &self,
        config: &storage::AppConfig,
        app_ctx: &RecordingContext,
        utterance: &str,
        operation_id: &str,
        voice_intent: crate::voice_intent::VoiceIntent,
    ) -> std::result::Result<AskVoiceDraftOutcome, String> {
        if voice_intent.kind != crate::voice_intent::VoiceIntentKind::DraftInsert {
            return Err("Ask draft execution requires a draft intent".to_string());
        }
        if self.current_state() != PipelineState::Idle {
            return Err("Another voice operation is already active".to_string());
        }
        let generation = self.session.begin();
        let mut cancelled = self.session.watch();

        let dictionary_store = self.app_handle.state::<storage::DictionaryStore>();
        let dictionary_words = dictionary_store.words().await;
        let correction_rules = dictionary_store
            .enabled_correction_rules()
            .await
            .into_iter()
            .map(|rule| llm::CorrectionRule {
                id: rule.id,
                pattern: rule.pattern,
                replacement: rule.replacement,
                enabled: rule.enabled,
            })
            .collect::<Vec<_>>();
        let session_token = if config.llm_provider == "cloud" {
            self.app_handle
                .state::<SessionTokenStore>()
                .0
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone()
        } else {
            String::new()
        };

        let outcome = tokio::select! {
            biased;
            _ = cancelled.wait_for(|value| *value != generation) => return Err("Draft cancelled".into()),
            outcome = self.polish_text(PolishTextInput {
                raw_text: utterance,
                voice_mode: crate::voice_intent::VoiceMode::Ask,
                config,
                app_ctx,
                dictionary_words,
                correction_rules,
                selected_text: None,
                session_token,
                operation_id: Some(operation_id.to_string()),
                voice_intent,
                popup_fallback_enabled: false,
                session_generation: generation,
            }) => outcome,
        };
        if !self.session.is_live(generation) {
            return Err("Draft cancelled".into());
        }
        self.set_state(PipelineState::Idle);

        let execution = outcome.voice_execution.ok_or_else(|| {
            "Draft was not generated; no application text was changed".to_string()
        })?;
        if outcome.final_text.trim().is_empty() {
            return Err("Draft generation returned empty output".to_string());
        }
        Ok(AskVoiceDraftOutcome {
            text: outcome.final_text,
            execution,
        })
    }

    /// Save the transcription to history.
    async fn save_history(
        &self,
        raw_text: &str,
        final_text: &str,
        app_ctx: &RecordingContext,
        duration_ms: Option<i64>,
        config: &storage::AppConfig,
        output: HistoryOutputMetadata,
    ) {
        let policy = config.history_retention_policy();
        if !policy.enabled {
            tracing::debug!("History save skipped because history is disabled");
            return;
        }

        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let scene_diagnostics = active_scene_history_diagnostics(config.active_scene.as_ref());
        let entry = storage::HistoryEntry {
            id: 0, // auto-increment
            created_at: now,
            context_profile_id: app_ctx.profile.id.clone(),
            context_label: app_ctx.profile.app_label.clone(),
            context_icon_key: app_ctx.profile.icon_key.clone(),
            context_family: app_ctx.profile.family,
            browser_access_status: app_ctx.browser_access_status,
            provider_kind: history_provider_kind(config),
            raw_text: raw_text.to_string(),
            polished_text: final_text.to_string(),
            language: None,
            duration_ms,
            active_scene_id: scene_diagnostics.id,
            active_scene_source: scene_diagnostics.source,
            active_scene_name: scene_diagnostics.name,
            active_scene_prompt_chars: scene_diagnostics.prompt_chars,
            active_scene_prompt_truncated: scene_diagnostics.prompt_truncated,
            output_status: output.status,
            output_error: output.error,
        };
        if let Err(e) = self
            .app_handle
            .state::<storage::HistoryStore>()
            .add_with_policy(entry, &policy)
            .await
        {
            tracing::error!("Failed to save history: {}", e);
        }
    }

    /// Starts the worker that types live-committed words into the recording target.
    /// Without a direct typing path (clipboard strategies, no accessibility) the whole
    /// text is output once on stop instead.
    fn spawn_live_insert_worker(
        &self,
        config: &storage::AppConfig,
    ) -> Option<mpsc::UnboundedSender<String>> {
        let app_ctx = self
            .preloaded_app_ctx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()?;
        let strategy = output::InsertionStrategy::from_config_value(&config.insertion_strategy)
            .direct_streaming_strategy()
            .filter(|strategy| {
                *strategy != output::InsertionStrategy::Keyboard
                    || output::keyboard::check_keyboard_available().is_ok()
            })
            .filter(|_| is_accessibility_trusted());
        let Some(strategy) = strategy else {
            tracing::warn!("Live typing has no direct insert path; text is output on stop");
            return None;
        };
        let worker = spawn_streaming_insert_worker(StreamingInsertWorkerContext {
            app_handle: self.app_handle.clone(),
            session: self.session.clone(),
            session_id: self.session.current(),
            context_detector: self.context_detector.clone(),
            strategy,
            windows_sendinput_options: output::windows_sendinput::WindowsSendInputOptions {
                newline_mode:
                    output::windows_sendinput::WindowsSendInputNewlineMode::from_config_value(
                        &config.windows_sendinput_newline_mode,
                    ),
            },
            expected_target_guard: app_ctx.target_guard.clone(),
            expected_target_label: app_ctx.profile.app_label.clone(),
        });
        let sender = worker.sender.clone();
        *self
            .live_insert_worker
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(worker);
        Some(sender)
    }

    /// Finishes a live typing session: types the words the worker has not inserted yet
    /// (normally just the last utterance's tail). No AI polish runs in this mode.
    async fn finish_live_typing(
        &self,
        raw_text: &str,
        app_ctx: &RecordingContext,
        config: &storage::AppConfig,
        session_generation: u64,
    ) -> PolishTextOutcome {
        let worker = self
            .live_insert_worker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let report = match worker {
            Some(worker) => worker.finish().await,
            None => None,
        };
        let Some(report) = report.filter(StreamingInsertReport::has_inserted_text) else {
            let output_result = self
                .output_text(
                    raw_text,
                    &app_ctx.profile.app_label,
                    &app_ctx.target_guard,
                    config,
                    session_generation,
                )
                .await;
            if let Err(error) = &output_result {
                tracing::error!("Output failed: {}", error);
                let _ = self
                    .app_handle
                    .emit("pipeline:error", output_user_error(error));
            }
            let output_metadata = history_output_metadata_for_output_result(
                output_result.as_ref(),
                output::InsertionStrategy::from_config_value(&config.insertion_strategy)
                    == output::InsertionStrategy::ClipboardCopyOnly,
            );
            return PolishTextOutcome {
                final_text: raw_text.to_string(),
                llm_elapsed: std::time::Duration::ZERO,
                history_output_status: output_metadata.status,
                history_output_error: output_metadata.error,
                voice_execution: None,
            };
        };

        let target_trusted = !report.failed
            && !report.target_lost
            && self
                .context_detector
                .target_still_matches_now(&app_ctx.target_guard);
        let mut report = report;
        match streaming_recovery_action(&report, Some(raw_text), true, target_trusted) {
            StreamingRecoveryAction::InsertSuffix { suffix } => {
                match output::output_stream_chunk(
                    &self.app_handle,
                    &suffix,
                    report.strategy,
                    output::windows_sendinput::WindowsSendInputOptions {
                        newline_mode:
                            output::windows_sendinput::WindowsSendInputNewlineMode::from_config_value(
                                &config.windows_sendinput_newline_mode,
                            ),
                    },
                )
                .await
                {
                    Ok(result) if result.chars_inserted == suffix.chars().count() => {
                        report.inserted_text.push_str(&suffix);
                    }
                    Ok(result) => {
                        report
                            .inserted_text
                            .extend(suffix.chars().take(result.chars_inserted));
                        report.failed = true;
                        report.error_message = result.message;
                    }
                    Err(error) => {
                        report.failed = true;
                        report.error_message = Some(error);
                    }
                }
                self.emit_streaming_insert_result(&report, &app_ctx.profile.app_label);
            }
            StreamingRecoveryAction::AlreadyComplete
            | StreamingRecoveryAction::NoRecoveryNeeded => {
                self.emit_streaming_insert_result(&report, &app_ctx.profile.app_label);
            }
            StreamingRecoveryAction::CopyFullToClipboard { reason }
            | StreamingRecoveryAction::CopyPartialToClipboard { reason } => {
                tracing::warn!("Live typing could not finish in place: {reason}");
                report.failed = true;
                report.error_message = Some(format!("Full result copied to clipboard: {reason}"));
                self.copy_text_to_clipboard_with_warning(
                    raw_text,
                    &app_ctx.profile.app_label,
                    config,
                    streaming_insert_user_error(report.error_message.clone()),
                )
                .await;
            }
        }
        let status = if report.failed { "partial" } else { "inserted" };
        PolishTextOutcome {
            final_text: raw_text.to_string(),
            llm_elapsed: std::time::Duration::ZERO,
            history_output_status: Some(status.to_string()),
            history_output_error: report.error_message.clone(),
            voice_execution: None,
        }
    }

    fn emit_streaming_insert_result(&self, report: &StreamingInsertReport, app_name: &str) {
        let chars_inserted = report.chars_inserted();
        let mut insert_result = if report.failed {
            output::InsertResult::partially_inserted(report.strategy, chars_inserted)
        } else {
            output::InsertResult::inserted(report.strategy, chars_inserted)
        };

        let warning = report
            .failed
            .then(|| streaming_insert_user_error(report.error_message.clone()));
        if let Some(user_error) = warning.as_ref() {
            insert_result = insert_result.with_warning(user_error);
        }

        tracing::info!(
            "Streaming insert completed with failed={}, chunks={}, chars_inserted={}",
            report.failed,
            report.attempted_chunks,
            chars_inserted
        );
        let _ = self
            .app_handle
            .emit("pipeline:insert_result", &insert_result);
        if let Some(user_error) = warning {
            let _ = self.app_handle.emit("pipeline:warning", &user_error);
        }
        let _ = self.app_handle.emit("pipeline:target_app", app_name);
    }

    fn retain_recovery(
        &self,
        text: &str,
        app_name: &str,
        generation: u64,
        reason: crate::recovery::RecoveryReason,
        chars_inserted: usize,
    ) {
        if self.session.is_live(generation) {
            crate::recovery::retain(
                &self.app_handle,
                generation,
                text,
                app_name,
                reason,
                chars_inserted,
            );
        }
    }

    async fn copy_streaming_recovery_to_clipboard(
        &self,
        text: &str,
        reason: String,
        app_name: &str,
        config: &storage::AppConfig,
        generation: u64,
        report: &StreamingInsertReport,
    ) {
        if !self.session.is_live(generation) {
            return;
        }
        let recovery_reason = if report.target_lost {
            crate::recovery::RecoveryReason::TargetChanged
        } else if report.has_inserted_text() {
            crate::recovery::RecoveryReason::PartialInsert
        } else {
            crate::recovery::RecoveryReason::OutputFailed
        };
        // Save before copying, so a clipboard failure cannot lose the result.
        self.retain_recovery(
            text,
            app_name,
            generation,
            recovery_reason,
            report.inserted_text.chars().count(),
        );
        self.copy_text_to_clipboard_with_warning(
            text,
            app_name,
            config,
            streaming_insert_user_error(Some(format!("Full result copied to clipboard: {reason}"))),
        )
        .await;
    }

    async fn copy_text_to_clipboard_with_warning(
        &self,
        text: &str,
        app_name: &str,
        config: &storage::AppConfig,
        warning: crate::error::UserError,
    ) {
        let clipboard_options = output::clipboard::ClipboardOutputOptions {
            restore_after_paste: false,
            paste_shortcut: output::clipboard::PasteShortcut::from_config_value(
                &config.paste_shortcut,
            ),
            auto_paste: false,
        };

        match output::output_with_strategy(
            &self.app_handle,
            text,
            output::InsertionStrategy::ClipboardCopyOnly,
            clipboard_options,
            output::windows_sendinput::WindowsSendInputOptions {
                newline_mode:
                    output::windows_sendinput::WindowsSendInputNewlineMode::from_config_value(
                        &config.windows_sendinput_newline_mode,
                    ),
            },
        )
        .await
        {
            Ok(mut outcome) => {
                outcome.insert_result = outcome.insert_result.with_warning(&warning);
                let _ = self
                    .app_handle
                    .emit("pipeline:insert_result", &outcome.insert_result);
                let _ = self.app_handle.emit("pipeline:warning", &warning);
                let _ = self.app_handle.emit("pipeline:target_app", app_name);
            }
            Err(error) => {
                tracing::error!("Failed to copy streaming recovery text to clipboard: {error}");
                let error = anyhow::anyhow!(error);
                let _ = self
                    .app_handle
                    .emit("pipeline:error", output_user_error(&error));
            }
        }
    }

    async fn restore_after_capsule_interaction(&self, target: &TargetAppGuard) {
        let Some(capsule) = self.app_handle.get_webview_window("capsule") else {
            return;
        };
        if !capsule.is_focused().unwrap_or(false) {
            return;
        }
        let detector = self.context_detector.clone();
        let target = target.clone();
        let _ = tokio::task::spawn_blocking(move || {
            if capsule.is_focused().unwrap_or(false) {
                detector.restore_target_application(&target);
            }
        })
        .await;
    }

    async fn output_text(
        &self,
        text: &str,
        app_name: &str,
        target_guard: &TargetAppGuard,
        config: &storage::AppConfig,
        session_generation: u64,
    ) -> Result<output::InsertResult> {
        if !self.session.is_live(session_generation) {
            anyhow::bail!("Output session was cancelled");
        }
        self.set_state(PipelineState::Outputting);

        self.restore_after_capsule_interaction(target_guard).await;
        if !self.session.is_live(session_generation) {
            anyhow::bail!("Output session was cancelled");
        }

        let configured_strategy =
            output::InsertionStrategy::from_config_value(&config.insertion_strategy);
        let target_warning = (strategy_uses_automated_input(configured_strategy)
            && !self.context_detector.target_still_matches_now(target_guard))
        .then(|| crate::error::UserError {
            code: "output_target_changed".to_string(),
            details: Some(
                "The target app changed before output; the full text was copied instead."
                    .to_string(),
            ),
            retry_count: 0,
        });
        let requested_strategy = if target_warning.is_some() {
            output::InsertionStrategy::ClipboardCopyOnly
        } else {
            configured_strategy
        };
        let (strategy, accessibility_warning) =
            effective_strategy_for_accessibility(requested_strategy, is_accessibility_trusted());

        // Linux: check keyboard availability before attempting
        let effective_strategy = if strategy.needs_keyboard_access() {
            if let Err(reason) = output::keyboard::check_keyboard_available() {
                if reason == "wayland_unsupported" {
                    tracing::warn!(
                        "Keyboard output not supported on Wayland, falling back to clipboard"
                    );
                    let ue = crate::error::UserError {
                        code: "output_wayland_unsupported".to_string(),
                        details: None,
                        retry_count: 0,
                    };
                    let _ = self.app_handle.emit("pipeline:warning", &ue);
                    output::InsertionStrategy::ClipboardPaste
                } else {
                    tracing::warn!("xdotool not found, keyboard output may fail: {}", reason);
                    strategy
                }
            } else {
                strategy
            }
        } else {
            strategy
        };

        let clipboard_options = output::clipboard::ClipboardOutputOptions {
            restore_after_paste: config.restore_clipboard_after_paste,
            paste_shortcut: output::clipboard::PasteShortcut::from_config_value(
                &config.paste_shortcut,
            ),
            auto_paste: true,
        };

        let mut output_outcome = match output::output_with_strategy(
            &self.app_handle,
            text,
            effective_strategy,
            clipboard_options,
            output::windows_sendinput::WindowsSendInputOptions {
                newline_mode:
                    output::windows_sendinput::WindowsSendInputNewlineMode::from_config_value(
                        &config.windows_sendinput_newline_mode,
                    ),
            },
        )
        .await
        {
            Ok(outcome) => outcome,
            Err(e) => {
                self.retain_recovery(
                    text,
                    app_name,
                    session_generation,
                    if target_warning.is_some() {
                        crate::recovery::RecoveryReason::TargetChanged
                    } else {
                        crate::recovery::RecoveryReason::OutputFailed
                    },
                    0,
                );
                anyhow::bail!("{}", e)
            }
        };

        if let Some(user_error) = target_warning.or(accessibility_warning) {
            output_outcome.insert_result = output_outcome.insert_result.with_warning(&user_error);
            output_outcome.warning.get_or_insert(user_error);
        }

        tracing::info!(
            "Output completed with status={:?}, strategy={:?}, chars_inserted={}",
            output_outcome.insert_result.status,
            output_outcome.insert_result.strategy_used,
            output_outcome.insert_result.chars_inserted
        );
        let insert_result = output_outcome.insert_result.clone();
        if let Some(reason) = crate::recovery::reason_for_result(
            &insert_result,
            configured_strategy == output::InsertionStrategy::ClipboardCopyOnly,
        ) {
            self.retain_recovery(
                text,
                app_name,
                session_generation,
                reason,
                insert_result.chars_inserted,
            );
        }
        let _ = self
            .app_handle
            .emit("pipeline:insert_result", &insert_result);

        if let Some(user_error) = output_outcome.warning {
            tracing::info!("Output completed with warning: {}", user_error.code);
            let _ = self.app_handle.emit("pipeline:warning", &user_error);
        }

        let _ = self.app_handle.emit("pipeline:target_app", app_name);
        Ok(insert_result)
    }

    async fn pre_warm_endpoint(&self, endpoint: &str, managed_cloud: bool) {
        tracing::debug!("Pre-warming HTTP connection to {}", endpoint);
        let request = self.shared_client.head(endpoint);
        let request = if managed_cloud {
            crate::with_desktop_client_version(request)
        } else {
            request
        };
        if let Err(error) = request
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            tracing::debug!("HTTP connection pre-warm did not complete: {error}");
        }
    }

    /// Pre-warm the active STT and LLM hosts concurrently so the next dictation can reuse TLS.
    /// Called after startup and whenever provider connection settings change.
    pub async fn pre_warm(&self) {
        let config = self.load_config().await;
        let warm_local_stt = cfg!(target_os = "windows")
            && is_managed_ultra_local_config(&config)
            && std::env::var_os("ULTRA_STT_ROOT")
                .is_some_and(|root| std::path::PathBuf::from(root).is_absolute());

        let stt_endpoint = match config.stt_provider.as_str() {
            "cloud" => {
                let base = crate::api_base_url();
                Some((format!("{}/api/proxy/stt", base), true))
            }
            "glm-asr" => Some((
                "https://open.bigmodel.cn/api/paas/v4/audio/transcriptions".to_string(),
                false,
            )),
            "openai-whisper" => Some((
                "https://api.openai.com/v1/audio/transcriptions".to_string(),
                false,
            )),
            "groq-whisper" => Some((
                "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
                false,
            )),
            "siliconflow" => Some((
                "https://api.siliconflow.cn/v1/audio/transcriptions".to_string(),
                false,
            )),
            "deepgram" => Some(("https://api.deepgram.com/v1/listen".to_string(), false)),
            "assemblyai" => Some((
                "https://api.assemblyai.com/v2/transcript".to_string(),
                false,
            )),
            stt::volcengine::VOLCENGINE_DOUBAO_PROVIDER => Some((
                "https://openspeech.bytedance.com/api/v3/sauc/bigmodel_async".to_string(),
                false,
            )),
            stt::config::CUSTOM_WHISPER_PROVIDER if warm_local_stt => None,
            stt::config::CUSTOM_WHISPER_PROVIDER => {
                stt::config::normalize_custom_whisper_endpoint(&config.stt_custom_base_url)
                    .ok()
                    .map(|endpoint| (endpoint, false))
            }
            _ => {
                tracing::debug!(
                    "Unknown STT provider '{}', skipping pre-warm",
                    config.stt_provider
                );
                None
            }
        };

        let llm_endpoint = if config.polish_enabled {
            if config.llm_provider == "cloud" {
                let base = crate::api_base_url();
                Some((format!("{}/api/proxy/llm", base), true))
            } else {
                crate::llm::protocol::chat_endpoint(&config.llm_provider, &config.llm_base_url)
                    .ok()
                    .map(|endpoint| (endpoint, false))
            }
        } else {
            None
        };

        let warm_stt = async {
            if warm_local_stt {
                if let Err(error) = stt::ultra_local::warm().await {
                    tracing::warn!("Local speech service pre-warm failed: {error}");
                }
            } else if let Some((endpoint, managed_cloud)) = stt_endpoint {
                self.pre_warm_endpoint(&endpoint, managed_cloud).await;
            }
        };
        let warm_llm = async {
            if let Some((endpoint, managed_cloud)) = llm_endpoint {
                self.pre_warm_endpoint(&endpoint, managed_cloud).await;
            }
        };
        tokio::join!(warm_stt, warm_llm);
        tracing::debug!("Provider connection pre-warm complete");
    }
}

fn is_managed_ultra_local_config(config: &storage::AppConfig) -> bool {
    config.stt_provider == stt::config::CUSTOM_WHISPER_PROVIDER
        && config.stt_custom_preset == "ultra-local"
        && config.stt_custom_base_url == stt::ultra_local::BASE
        && config.stt_custom_model == stt::ultra_local::MODEL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_enter_and_escape_keep_recording_controls_without_blocking_idle_start() {
        let escape = Shortcut::new(None, Code::Escape);
        let enter = Shortcut::new(None, Code::Enter);
        assert_eq!(
            recording_control_key(&escape, PipelineState::Idle, false),
            None
        );
        assert_eq!(
            recording_control_key(&enter, PipelineState::Idle, false),
            None
        );
        assert_eq!(
            recording_control_key(&escape, PipelineState::Recording, false),
            Some(Code::Escape)
        );
        assert_eq!(
            recording_control_key(&escape, PipelineState::Idle, true),
            Some(Code::Escape)
        );
        assert_eq!(
            recording_control_key(&enter, PipelineState::Recording, false),
            Some(Code::Enter)
        );
        assert_eq!(
            recording_control_key(
                &Shortcut::new(
                    Some(tauri_plugin_global_shortcut::Modifiers::CONTROL),
                    Code::Enter
                ),
                PipelineState::Recording,
                false
            ),
            None
        );
    }
    use std::sync::atomic::AtomicBool;

    #[test]
    fn local_cuda_pre_warm_requires_exact_managed_config() {
        let mut config = storage::AppConfig::new_ultra_local_profile();
        assert!(is_managed_ultra_local_config(&config));
        config.stt_custom_preset = stt::config::ULTRA_REMOTE_STT_PRESET.to_string();
        assert!(!is_managed_ultra_local_config(&config));
        config.stt_custom_preset = "ultra-local".to_string();
        config.stt_custom_model = "other-model".to_string();
        assert!(!is_managed_ultra_local_config(&config));
    }

    #[test]
    fn escape_cancel_event_is_consumed_before_the_global_dictation_handler() {
        let pending = AtomicBool::new(false);
        let escape = Shortcut::new(None, Code::Escape);
        let dictation = Shortcut::new(
            Some(tauri_plugin_global_shortcut::Modifiers::CONTROL),
            Code::KeyR,
        );

        assert!(!consume_escape_event(&pending, &escape));
        pending.store(true, Ordering::SeqCst);
        assert!(!consume_escape_event(&pending, &dictation));
        assert!(consume_escape_event(&pending, &escape));
        assert!(!consume_escape_event(&pending, &escape));
    }

    #[test]
    fn enter_event_is_consumed_once_without_modifiers() {
        let pending = AtomicBool::new(true);
        let enter = Shortcut::new(None, Code::Enter);
        let shifted = Shortcut::new(
            Some(tauri_plugin_global_shortcut::Modifiers::SHIFT),
            Code::Enter,
        );
        assert!(!consume_enter_event(&pending, &shifted));
        assert!(consume_enter_event(&pending, &enter));
        assert!(!consume_enter_event(&pending, &enter));
    }

    #[test]
    fn remote_polish_options_only_follow_the_personal_remote_preset() {
        let mut config = storage::AppConfig::default();
        config.polish_style = "original".to_string();
        config.polish_enabled = false;
        assert!(remote_polish_options(&config).is_none());
        config.stt_provider = stt::config::CUSTOM_WHISPER_PROVIDER.to_string();
        config.stt_custom_preset = stt::config::ULTRA_REMOTE_STT_PRESET.to_string();
        assert_eq!(
            remote_polish_options(&config),
            Some(stt::RemotePolishOptions {
                style: "original".to_string(),
                enabled: false
            })
        );
        config.stt_custom_preset = "custom".to_string();
        assert!(remote_polish_options(&config).is_none());
    }

    #[test]
    fn output_user_error_preserves_accessibility_required() {
        let err = anyhow::anyhow!("Output failed: ACCESSIBILITY_REQUIRED");
        let user_error = output_user_error(&err);

        assert_eq!(user_error.code, "accessibility_required");
        assert_eq!(user_error.details, None);
    }

    #[test]
    fn output_user_error_keeps_non_permission_details() {
        let err = anyhow::anyhow!("Both keyboard and clipboard output failed");
        let user_error = output_user_error(&err);

        assert_eq!(user_error.code, "output_failed");
        assert_eq!(
            user_error.details.as_deref(),
            Some("Both keyboard and clipboard output failed")
        );
    }

    #[test]
    fn history_metadata_records_successful_insert() {
        let metadata = history_output_metadata_for_insert_result(
            &output::InsertResult::inserted(output::InsertionStrategy::Keyboard, 5),
            false,
        );

        assert_eq!(metadata.status.as_deref(), Some("inserted"));
        assert!(metadata.error.is_none());
    }

    #[test]
    fn history_metadata_records_clipboard_fallback_warning() {
        let warning = crate::error::UserError {
            code: "output_fallback_clipboard".to_string(),
            details: Some("wtype failed".to_string()),
            retry_count: 0,
        };
        let result =
            output::InsertResult::copied_fallback(output::InsertionStrategy::ClipboardCopyOnly, 5)
                .with_warning(&warning);

        let metadata = history_output_metadata_for_insert_result(&result, false);

        assert_eq!(metadata.status.as_deref(), Some("clipboard_fallback"));
        assert_eq!(metadata.error.as_deref(), Some("wtype failed"));
    }

    #[test]
    fn history_metadata_records_fallback_even_when_clipboard_paste_succeeds() {
        let warning = crate::error::UserError {
            code: "output_fallback_clipboard".to_string(),
            details: Some("keyboard failed".to_string()),
            retry_count: 0,
        };
        let result = output::InsertResult::inserted(output::InsertionStrategy::ClipboardPaste, 5)
            .with_warning(&warning);

        let metadata = history_output_metadata_for_insert_result(&result, false);

        assert_eq!(metadata.status.as_deref(), Some("fallback"));
        assert_eq!(metadata.error.as_deref(), Some("keyboard failed"));
    }

    #[test]
    fn history_metadata_keeps_intentional_clipboard_copy_normal() {
        let result =
            output::InsertResult::copied_fallback(output::InsertionStrategy::ClipboardCopyOnly, 5);

        let metadata = history_output_metadata_for_insert_result(&result, true);

        assert_eq!(metadata.status.as_deref(), Some("copied"));
        assert!(metadata.error.is_none());
    }

    #[test]
    fn history_metadata_records_partial_insert() {
        let metadata = history_output_metadata_for_insert_result(
            &output::InsertResult::partially_inserted(
                output::InsertionStrategy::WindowsSendInput,
                2,
            ),
            false,
        );

        assert_eq!(metadata.status.as_deref(), Some("partial"));
        assert!(metadata.error.is_none());
    }

    #[test]
    fn history_metadata_records_failed_clipboard_paste_as_fallback() {
        let result =
            output::InsertResult::copied_fallback(output::InsertionStrategy::ClipboardPaste, 5);

        let metadata = history_output_metadata_for_insert_result(&result, false);

        assert_eq!(metadata.status.as_deref(), Some("clipboard_fallback"));
        assert!(metadata.error.is_none());
    }

    #[test]
    fn history_metadata_records_failed_output() {
        let result = output::InsertResult::failed(output::InsertionStrategy::Keyboard);

        let metadata = history_output_metadata_for_insert_result(&result, false);

        assert_eq!(metadata.status.as_deref(), Some("failed"));
        assert!(metadata.error.is_none());
    }

    #[test]
    fn history_metadata_records_output_error() {
        let error = anyhow::anyhow!("Both keyboard and clipboard output failed");

        let metadata = history_output_metadata_for_output_result(Err(&error), false);

        assert_eq!(metadata.status.as_deref(), Some("failed"));
        assert_eq!(
            metadata.error.as_deref(),
            Some("Both keyboard and clipboard output failed")
        );
    }

    #[test]
    fn history_metadata_keeps_actual_output_status_after_an_earlier_failure() {
        let result = output::InsertResult::partially_inserted(
            output::InsertionStrategy::WindowsSendInput,
            2,
        );

        let metadata = history_output_metadata_with_prior_error(
            Ok(&result),
            false,
            "LLM polish failed: provider unavailable",
        );

        assert_eq!(metadata.status.as_deref(), Some("partial"));
        assert_eq!(
            metadata.error.as_deref(),
            Some("LLM polish failed: provider unavailable; output raw text")
        );
    }

    #[test]
    fn history_metadata_keeps_prior_failure_visible_after_raw_insert_succeeds() {
        let result = output::InsertResult::inserted(output::InsertionStrategy::Keyboard, 5);

        let metadata = history_output_metadata_with_prior_error(
            Ok(&result),
            false,
            "LLM polish failed: provider unavailable",
        );

        assert_eq!(metadata.status.as_deref(), Some("fallback"));
        assert_eq!(
            metadata.error.as_deref(),
            Some("LLM polish failed: provider unavailable; output raw text")
        );
    }

    #[test]
    fn history_metadata_keeps_both_failures_when_raw_output_also_fails() {
        let output_error = anyhow::anyhow!("clipboard unavailable");

        let metadata = history_output_metadata_with_prior_error(
            Err(&output_error),
            false,
            "LLM polish failed: provider unavailable",
        );

        assert_eq!(metadata.status.as_deref(), Some("failed"));
        assert_eq!(
            metadata.error.as_deref(),
            Some("LLM polish failed: provider unavailable; raw output: clipboard unavailable")
        );
    }

    #[test]
    fn accessibility_fallback_copies_when_auto_requires_accessibility() {
        let (strategy, warning) =
            effective_strategy_for_accessibility(output::InsertionStrategy::Auto, false);

        assert_eq!(strategy, output::InsertionStrategy::ClipboardCopyOnly);
        assert_eq!(
            warning.as_ref().map(|warning| warning.code.as_str()),
            Some("accessibility_required")
        );
    }

    #[test]
    fn accessibility_fallback_copies_when_clipboard_paste_requires_accessibility() {
        let (strategy, warning) =
            effective_strategy_for_accessibility(output::InsertionStrategy::ClipboardPaste, false);

        assert_eq!(strategy, output::InsertionStrategy::ClipboardCopyOnly);
        assert_eq!(
            warning.as_ref().map(|warning| warning.code.as_str()),
            Some("accessibility_required")
        );
    }

    #[test]
    fn accessibility_fallback_keeps_strategy_when_accessibility_is_trusted() {
        let (strategy, warning) =
            effective_strategy_for_accessibility(output::InsertionStrategy::Keyboard, true);

        assert_eq!(strategy, output::InsertionStrategy::Keyboard);
        assert!(warning.is_none());
    }

    #[test]
    fn accessibility_fallback_keeps_copy_only_without_accessibility() {
        let (strategy, warning) = effective_strategy_for_accessibility(
            output::InsertionStrategy::ClipboardCopyOnly,
            false,
        );

        assert_eq!(strategy, output::InsertionStrategy::ClipboardCopyOnly);
        assert!(warning.is_none());
    }

    #[test]
    fn copy_only_output_does_not_require_the_target_window() {
        assert!(!strategy_uses_automated_input(
            output::InsertionStrategy::ClipboardCopyOnly
        ));
        assert!(strategy_uses_automated_input(
            output::InsertionStrategy::ClipboardPaste
        ));
    }

    #[test]
    fn streaming_insert_strategy_defaults_to_disabled() {
        let config = storage::AppConfig::default();

        assert_eq!(
            streaming_insert_strategy_for_config(&config, false, true, true),
            None
        );
    }

    #[test]
    fn streaming_insert_strategy_uses_keyboard_for_auto_when_safe() {
        let mut config = storage::AppConfig {
            streaming_insert_enabled: true,
            ..storage::AppConfig::default()
        };
        config.insertion_strategy = "auto".to_string();

        assert_eq!(
            streaming_insert_strategy_for_config(&config, false, true, true),
            Some(output::InsertionStrategy::Keyboard)
        );
    }

    #[test]
    fn streaming_insert_strategy_rejects_selected_text_and_clipboard_paths() {
        let mut config = storage::AppConfig {
            streaming_insert_enabled: true,
            ..storage::AppConfig::default()
        };

        assert_eq!(
            streaming_insert_strategy_for_config(&config, true, true, true),
            None
        );

        config.insertion_strategy = "clipboardPaste".to_string();
        assert_eq!(
            streaming_insert_strategy_for_config(&config, false, true, true),
            None
        );
    }

    #[test]
    fn streaming_insert_strategy_requires_accessibility_and_keyboard_availability() {
        let config = storage::AppConfig {
            streaming_insert_enabled: true,
            ..storage::AppConfig::default()
        };

        assert_eq!(
            streaming_insert_strategy_for_config(&config, false, false, true),
            None
        );
        assert_eq!(
            streaming_insert_strategy_for_config(&config, false, true, false),
            None
        );
    }

    #[test]
    fn streaming_insert_strategy_allows_windows_sendinput_without_keyboard_check() {
        let mut config = storage::AppConfig {
            streaming_insert_enabled: true,
            ..storage::AppConfig::default()
        };
        config.insertion_strategy = "windowsSendInput".to_string();

        assert_eq!(
            streaming_insert_strategy_for_config(&config, false, true, false),
            Some(output::InsertionStrategy::WindowsSendInput)
        );
    }

    fn streaming_report_for_test(inserted_text: &str, failed: bool) -> StreamingInsertReport {
        StreamingInsertReport {
            strategy: output::InsertionStrategy::Keyboard,
            inserted_text: inserted_text.to_string(),
            attempted_chunks: 1,
            failed,
            target_lost: false,
            error_message: failed.then(|| "streaming failed".to_string()),
        }
    }

    #[test]
    fn streaming_target_guard_blocks_process_changes() {
        let expected = TargetAppGuard {
            process_id: Some(42),
            native_identity: Some("com.example.notes".to_string()),
            ..TargetAppGuard::default()
        };
        assert!(expected.matches(&expected));
        assert!(!expected.matches(&TargetAppGuard {
            process_id: Some(99),
            native_identity: Some("com.example.browser".to_string()),
            ..TargetAppGuard::default()
        }));
    }

    #[test]
    fn streaming_recovery_copies_full_when_target_is_not_trusted() {
        let report = streaming_report_for_test("Hello", true);

        assert_eq!(
            streaming_recovery_action(&report, Some("Hello world"), true, false),
            StreamingRecoveryAction::CopyFullToClipboard {
                reason: "target app changed after partial streaming insert".to_string()
            }
        );
    }

    #[test]
    fn streaming_recovery_uses_char_safe_suffix_for_cjk_text() {
        let report = streaming_report_for_test("你好", true);

        assert_eq!(
            streaming_recovery_action(&report, Some("你好，世界"), true, true),
            StreamingRecoveryAction::InsertSuffix {
                suffix: "，世界".to_string()
            }
        );
    }

    #[test]
    fn streaming_recovery_copies_partial_when_llm_fails_after_insert() {
        let report = streaming_report_for_test("partial", true);

        assert_eq!(
            streaming_recovery_action(&report, None, false, false),
            StreamingRecoveryAction::CopyPartialToClipboard {
                reason: "LLM failed after partial streaming insert".to_string()
            }
        );
    }

    #[test]
    fn pipeline_start_options_force_translation_for_current_run_only() {
        let config = storage::AppConfig {
            translate_enabled: false,
            target_lang: "ja".to_string(),
            translation: storage::TranslationConfig {
                targets: vec!["ja".to_string()],
                active_target: "ja".to_string(),
            },
            ..storage::AppConfig::default()
        };

        let next_config = apply_pipeline_start_options(
            config.clone(),
            PipelineStartOptions {
                force_translate: true,
                live_typing: false,
            },
        );

        assert!(next_config.translate_enabled);
        assert_eq!(next_config.target_lang, "ja");
        assert!(!config.translate_enabled);
    }

    #[test]
    fn switch_translation_target_updates_capture_without_restart_and_freezes_at_finalization() {
        let mut operation = TranslationOperationState::new("ja".to_string());
        let previous = operation.switch_target("fr".to_string()).unwrap();
        assert_eq!(previous, "ja");
        assert_eq!(operation.phase, TranslationOperationPhase::Capturing);
        assert_eq!(operation.finalize(), "fr");
        assert_eq!(operation.phase, TranslationOperationPhase::Finalizing);
        assert_eq!(
            operation.switch_target("de".to_string()),
            Err("translation_operation_finished")
        );
        assert_eq!(operation.target, "fr");
    }

    #[test]
    fn selected_text_output_policy_copies_non_destructive_questions() {
        let mut config = storage::AppConfig {
            stt_language: "zh-Hans".to_string(),
            ..Default::default()
        };
        let chinese = route_pipeline_voice_intent(
            crate::voice_intent::VoiceMode::Dictate,
            "这段是什么意思",
            Some("selected text"),
            &config,
        );
        assert_eq!(
            chinese.placement,
            crate::voice_intent::VoiceOutputPlacement::PopupAnswer
        );
        config.stt_language = "en".to_string();
        let english = route_pipeline_voice_intent(
            crate::voice_intent::VoiceMode::Dictate,
            "summarize this",
            Some("selected text"),
            &config,
        );
        assert_eq!(
            english.placement,
            crate::voice_intent::VoiceOutputPlacement::PopupAnswer
        );
    }

    #[test]
    fn selected_text_output_policy_replaces_for_explicit_editing() {
        let mut config = storage::AppConfig {
            stt_language: "zh-Hans".to_string(),
            ..Default::default()
        };
        let rewrite = route_pipeline_voice_intent(
            crate::voice_intent::VoiceMode::Dictate,
            "润色这段",
            Some("selected text"),
            &config,
        );
        assert_eq!(
            rewrite.placement,
            crate::voice_intent::VoiceOutputPlacement::ReplaceSelection
        );
        config.stt_language = "en".to_string();
        let translation = route_pipeline_voice_intent(
            crate::voice_intent::VoiceMode::Dictate,
            "translate this to English",
            Some("selected text"),
            &config,
        );
        assert_eq!(
            translation.placement,
            crate::voice_intent::VoiceOutputPlacement::ReplaceSelection
        );
    }

    #[test]
    fn selected_text_without_llm_polish_blocks_raw_instruction_output() {
        assert!(selected_text_command_requires_llm(Some("selected text")));
        assert!(!selected_text_command_requires_llm(None));
        assert!(!selected_text_command_requires_llm(Some(" \n\t ")));
    }

    #[test]
    fn shared_voice_router_pipeline_keeps_discussed_commands_nondestructive() {
        let config = storage::AppConfig {
            stt_language: "en".to_string(),
            ..Default::default()
        };

        let ordinary = route_pipeline_voice_intent(
            crate::voice_intent::VoiceMode::Dictate,
            "I need to draft tomorrow",
            None,
            &config,
        );
        assert_eq!(
            ordinary.kind,
            crate::voice_intent::VoiceIntentKind::DictateInsert
        );

        let selected = route_pipeline_voice_intent(
            crate::voice_intent::VoiceMode::Dictate,
            "do not rewrite this",
            Some("selected text"),
            &config,
        );
        assert_eq!(
            selected.kind,
            crate::voice_intent::VoiceIntentKind::AskSelection
        );
        assert_eq!(
            selected.placement,
            crate::voice_intent::VoiceOutputPlacement::PopupAnswer
        );
    }

    #[test]
    fn session_error_latch_returns_matching_error() {
        let latch = Mutex::new(Some((
            7,
            crate::error::UserError {
                code: "stt_quota_exceeded".to_string(),
                details: Some("quota".to_string()),
                retry_count: 0,
            },
        )));

        let err = take_matching_stt_error(&latch, 7).unwrap();
        assert_eq!(err.code, "stt_quota_exceeded");
        assert!(latch.lock().unwrap().is_none());
    }

    #[test]
    fn session_error_latch_ignores_stale_error() {
        let latch = Mutex::new(Some((
            6,
            crate::error::UserError {
                code: "stt_quota_exceeded".to_string(),
                details: Some("quota".to_string()),
                retry_count: 0,
            },
        )));

        assert!(take_matching_stt_error(&latch, 7).is_none());
        assert!(latch.lock().unwrap().is_some());
    }

    #[test]
    fn send_audio_error_is_latched_only_for_the_active_session() {
        let session = SessionClock::new();
        let stale = session.begin();
        let active = session.begin();
        let latch = Mutex::new(None);
        let error = crate::error::AppError::Network("socket closed".to_string());

        let user_error = latch_stt_task_error_if_active(&session, &latch, active, &error)
            .expect("active send failure should be surfaced");

        assert_eq!(user_error.code, "stt_timeout");
        assert_eq!(
            take_matching_stt_error(&latch, active)
                .expect("matching error should be consumable")
                .code,
            "stt_timeout"
        );

        assert!(latch_stt_task_error_if_active(&session, &latch, stale, &error).is_none());
        session.end();
        assert!(latch_stt_task_error_if_active(&session, &latch, active, &error).is_none());
    }

    #[test]
    fn no_speech_user_error_has_localizable_code() {
        let err = no_speech_user_error();
        assert_eq!(err.code, "stt_no_speech_detected");
        assert_eq!(err.retry_count, 0);
    }

    #[test]
    fn llm_polish_user_error_keeps_quota_code() {
        let err = llm_polish_user_error(&crate::error::AppError::LlmQuota(
            "quota exceeded".to_string(),
        ));
        assert_eq!(err.code, "llm_quota_exceeded");
        assert_eq!(err.details.as_deref(), Some("quota exceeded"));
    }

    #[test]
    fn llm_polish_user_error_uses_localizable_fallback_code() {
        let err = llm_polish_user_error(&crate::error::AppError::Auth(
            "Cloud LLM access denied".to_string(),
        ));
        assert_eq!(err.code, "llm_failed");
        assert_eq!(
            err.details.as_deref(),
            Some("Auth error: Cloud LLM access denied")
        );
    }

    #[test]
    fn active_scene_history_diagnostics_record_scene_metadata() {
        let scene = storage::ActiveScene {
            id: "builtin_meeting_notes".to_string(),
            source: "builtin".to_string(),
            name: "Meeting Notes".to_string(),
            prompt_template: format!("{}{}", "x".repeat(4_000), "overflow"),
        };

        let diagnostics = active_scene_history_diagnostics(Some(&scene));

        assert_eq!(diagnostics.id.as_deref(), Some("builtin_meeting_notes"));
        assert_eq!(diagnostics.source.as_deref(), Some("builtin"));
        assert_eq!(diagnostics.name.as_deref(), Some("Meeting Notes"));
        assert_eq!(diagnostics.prompt_chars, Some(4_000));
        assert!(diagnostics.prompt_truncated);
    }

    #[test]
    fn active_scene_history_diagnostics_are_empty_without_scene() {
        let diagnostics = active_scene_history_diagnostics(None);

        assert_eq!(diagnostics.id, None);
        assert_eq!(diagnostics.source, None);
        assert_eq!(diagnostics.name, None);
        assert_eq!(diagnostics.prompt_chars, None);
        assert!(!diagnostics.prompt_truncated);
    }

    #[test]
    fn history_provider_kind_uses_only_provider_classification() {
        let mut config = storage::AppConfig {
            polish_enabled: true,
            llm_provider: "cloud".to_string(),
            ..Default::default()
        };
        assert_eq!(
            history_provider_kind(&config),
            storage::HistoryProviderKind::ManagedCloud
        );

        config.llm_provider = "openrouter".to_string();
        assert_eq!(
            history_provider_kind(&config),
            storage::HistoryProviderKind::Byok
        );

        config.llm_provider = "ollama".to_string();
        assert_eq!(
            history_provider_kind(&config),
            storage::HistoryProviderKind::Local
        );
    }
}
