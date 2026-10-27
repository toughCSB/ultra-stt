//! Dictation session scenarios, driven through `PipelineHandle` with fake ports: no Tauri app,
//! microphone, recognizer or keyboard is involved.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{mpsc, Notify};

use super::capture::{Capture, LiveHints};
use super::phases::testing::{Note, RecordingNotifier};
use super::{
    AskWindows, LlmProviders, Microphone, Recorder, SessionPorts, SessionSettings, SessionStores,
    StateCell, SttProviders, TextInput,
};
use crate::app_detector::platform::ContextSignalSource;
use crate::app_detector::registry::AppRegistry;
use crate::app_detector::types::{ContextSignals, TargetAppGuard};
use crate::app_detector::ContextDetectorHandle;
use crate::error::AppError;
use crate::llm::{CorrectionRule, LlmProvider};
use crate::output::clipboard::ClipboardOutputOptions;
use crate::output::windows_sendinput::WindowsSendInputOptions;
use crate::output::{InsertResult, InsertionStrategy, OutputOutcome};
use crate::pipeline::{PipelineHandle, PipelineState};
use crate::recording_archive::{RecordingArchive, RecordingJournal};
use crate::recording_deadline::CaptureReadyAt;
use crate::recovery::RecoveryReason;
use crate::storage::{
    AppConfig, DictionaryStore, HistoryEntry, HistoryRetentionPolicy, HistoryStore,
};
use crate::stt::whisper_compat::WhisperCompatConfig;
use crate::stt::{SttConfig, SttProvider, TranscriptEvent};

// ─── Fakes ──────────────────────────────────────────────────────────────

struct MemorySettings(AppConfig);

#[async_trait]
impl SessionSettings for MemorySettings {
    async fn load_config(&self) -> AppConfig {
        self.0.clone()
    }
    fn cloud_session_token(&self) -> String {
        String::new()
    }
    fn stt_secret(&self, _config: &AppConfig) -> anyhow::Result<String> {
        Ok("test-key".into())
    }
    fn llm_secret(&self, _config: &AppConfig) -> anyhow::Result<String> {
        Ok("test-key".into())
    }
}

/// The real dictionary, history and recovery-audio stores, opened on a temp folder.
struct TempStores {
    root: PathBuf,
    dictionary: DictionaryStore,
    history: HistoryStore,
    archive: RecordingArchive,
    recovered: Mutex<Vec<(String, RecoveryReason)>>,
}

impl TempStores {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ultra-stt-session-scenario-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self {
            dictionary: DictionaryStore::new(root.join("dictionary.db")).unwrap(),
            history: HistoryStore::new(root.join("history.db")).unwrap(),
            archive: RecordingArchive::new(root.join("recordings")),
            recovered: Mutex::new(Vec::new()),
            root,
        }
    }

    async fn history_texts(&self) -> Vec<String> {
        self.history
            .list(10, 0)
            .await
            .unwrap()
            .into_iter()
            .map(|entry| entry.polished_text)
            .collect()
    }

    fn saved_recordings(&self) -> usize {
        self.archive
            .entries()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }
}

impl Drop for TempStores {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[async_trait]
impl SessionStores for TempStores {
    async fn dictionary_words(&self) -> Vec<String> {
        self.dictionary.words().await
    }
    async fn correction_rules(&self) -> Vec<CorrectionRule> {
        super::correction_rules_for_polish(self.dictionary.enabled_correction_rules().await)
    }
    fn begin_recording(&self) -> std::io::Result<RecordingJournal> {
        self.archive.begin()
    }
    async fn add_history(
        &self,
        entry: HistoryEntry,
        policy: &HistoryRetentionPolicy,
    ) -> anyhow::Result<()> {
        self.history.add_with_policy(entry, policy).await
    }
    fn retain_for_recovery(
        &self,
        _session_id: u64,
        text: &str,
        _app_name: &str,
        reason: RecoveryReason,
        _chars_inserted: usize,
    ) {
        self.recovered
            .lock()
            .unwrap()
            .push((text.to_string(), reason));
    }
}

/// A microphone that delivers one chunk of silence per recording. `hold_ready` keeps the
/// next capture "warming up" until `release_ready` is called.
#[derive(Default)]
struct FakeMicrophone {
    hold_ready: Mutex<Option<Arc<Notify>>>,
}

impl FakeMicrophone {
    fn hold_ready(&self) -> Arc<Notify> {
        let gate = Arc::new(Notify::new());
        *self.hold_ready.lock().unwrap() = Some(gate.clone());
        gate
    }
}

struct FakeRecorder {
    audio: Option<mpsc::Sender<Vec<u8>>>,
    ready_gate: Option<Arc<Notify>>,
}

#[async_trait]
impl Recorder for FakeRecorder {
    async fn wait_until_ready(&mut self) -> anyhow::Result<CaptureReadyAt> {
        if let Some(gate) = self.ready_gate.take() {
            gate.notified().await;
        }
        Ok(CaptureReadyAt::now())
    }
    fn stop(&mut self) {
        // Closing the channel is how the recognizer learns the recording ended.
        self.audio = None;
    }
    fn volume(&self) -> f32 {
        0.0
    }
    fn runtime_error(&self) -> Option<String> {
        None
    }
}

impl Microphone for FakeMicrophone {
    fn start(&self) -> anyhow::Result<Capture> {
        let (sender, receiver) = mpsc::channel(8);
        sender.try_send(vec![0; 320]).unwrap();
        let recorder = FakeRecorder {
            audio: Some(sender),
            ready_gate: self.hold_ready.lock().unwrap().take(),
        };
        Ok((Box::new(recorder), receiver))
    }
}

/// Recognizers that return the queued transcripts in order. A transcript can be held back
/// until its gate is opened, to keep a session in Transcribing.
#[derive(Default)]
struct FakeRecognizers {
    queue: Mutex<VecDeque<(String, Option<Arc<Notify>>)>>,
}

impl FakeRecognizers {
    fn will_hear(&self, transcript: &str) {
        self.queue
            .lock()
            .unwrap()
            .push_back((transcript.to_string(), None));
    }

    fn will_hear_after_gate(&self, transcript: &str) -> Arc<Notify> {
        let gate = Arc::new(Notify::new());
        self.queue
            .lock()
            .unwrap()
            .push_back((transcript.to_string(), Some(gate.clone())));
        gate
    }
}

struct FakeRecognizer {
    transcript: String,
    gate: Option<Arc<Notify>>,
}

#[async_trait]
impl SttProvider for FakeRecognizer {
    async fn connect(&mut self, _config: &SttConfig) -> Result<(), AppError> {
        Ok(())
    }
    async fn send_audio(&mut self, _chunk: &[u8]) -> Result<(), AppError> {
        Ok(())
    }
    async fn recv_transcript(&mut self) -> Result<Option<TranscriptEvent>, AppError> {
        std::future::pending().await
    }
    async fn disconnect(&mut self) -> Result<Option<String>, AppError> {
        if let Some(gate) = self.gate.take() {
            gate.notified().await;
        }
        Ok(Some(self.transcript.clone()))
    }
    fn name(&self) -> &str {
        "fake"
    }
}

impl SttProviders for FakeRecognizers {
    fn create(
        &self,
        _config: &AppConfig,
        _stt_config: &SttConfig,
        _custom_whisper: Option<WhisperCompatConfig>,
        _live_hints: Option<LiveHints>,
    ) -> Result<Box<dyn SttProvider>, AppError> {
        let (transcript, gate) = self
            .queue
            .lock()
            .unwrap()
            .pop_front()
            .expect("a transcript was queued for this recording");
        Ok(Box::new(FakeRecognizer { transcript, gate }))
    }
}

struct NoPolish;

impl LlmProviders for NoPolish {
    fn create(&self, _provider_name: &str) -> Box<dyn LlmProvider> {
        panic!("these scenarios run with AI polish off")
    }
}

/// Records what was typed, and with which strategy.
#[derive(Default)]
struct FakeKeyboard {
    typed: Mutex<Vec<(String, InsertionStrategy)>>,
}

impl FakeKeyboard {
    fn typed(&self) -> Vec<(String, InsertionStrategy)> {
        self.typed.lock().unwrap().clone()
    }
}

#[async_trait]
impl TextInput for FakeKeyboard {
    async fn insert(
        &self,
        text: &str,
        strategy: InsertionStrategy,
        _clipboard: ClipboardOutputOptions,
        _sendinput: WindowsSendInputOptions,
    ) -> Result<OutputOutcome, String> {
        self.typed
            .lock()
            .unwrap()
            .push((text.to_string(), strategy));
        let chars = text.chars().count();
        let insert_result = if strategy == InsertionStrategy::ClipboardCopyOnly {
            InsertResult::copied_fallback(strategy, chars)
        } else {
            InsertResult::inserted(strategy, chars)
        };
        Ok(OutputOutcome {
            insert_result,
            warning: None,
        })
    }
    async fn insert_chunk(
        &self,
        chunk: &str,
        strategy: InsertionStrategy,
        _sendinput: WindowsSendInputOptions,
    ) -> Result<InsertResult, String> {
        self.typed
            .lock()
            .unwrap()
            .push((chunk.to_string(), strategy));
        Ok(InsertResult::inserted(strategy, chunk.chars().count()))
    }
    fn selected_text(&self) -> Option<String> {
        None
    }
}

struct NoAsk;

impl AskWindows for NoAsk {
    fn busy(&self) -> bool {
        false
    }
    fn abort(&self) {}
    fn hide_answer(&self) {}
    fn show_answer(
        &self,
        _question: String,
        _answer: String,
        _intent: crate::voice_intent::VoiceIntentKind,
        _used_selected_text: bool,
        _selected_text_truncated: bool,
    ) -> Result<(), String> {
        Ok(())
    }
    fn show_error(&self, _message: String) -> Result<(), String> {
        Ok(())
    }
}

/// The app the user is typing into. `switch_to` simulates the user changing apps.
struct FakeTarget(Mutex<ContextSignals>);

impl FakeTarget {
    fn app(process_id: u32, identity: &str) -> ContextSignals {
        ContextSignals {
            process_id: Some(process_id),
            native_identity: Some(identity.to_string()),
            process_alias: Some(identity.to_string()),
            ..ContextSignals::default()
        }
    }

    fn switch_to(&self, signals: ContextSignals) {
        *self.0.lock().unwrap() = signals;
    }
}

impl ContextSignalSource for FakeTarget {
    fn collect(&self) -> Option<ContextSignals> {
        Some(self.0.lock().unwrap().clone())
    }
    fn current_target_guard(&self) -> Option<TargetAppGuard> {
        Some(TargetAppGuard::from(&*self.0.lock().unwrap()))
    }
}

// ─── Harness ────────────────────────────────────────────────────────────

struct Scenario {
    session: PipelineHandle,
    screen: Arc<RecordingNotifier>,
    stores: Arc<TempStores>,
    microphone: Arc<FakeMicrophone>,
    recognizers: Arc<FakeRecognizers>,
    keyboard: Arc<FakeKeyboard>,
    target: Arc<FakeTarget>,
}

impl Scenario {
    fn new() -> Self {
        let mut config = AppConfig::default();
        config.polish_enabled = false;
        let screen = Arc::new(RecordingNotifier::default());
        let stores = Arc::new(TempStores::new());
        let microphone = Arc::new(FakeMicrophone::default());
        let recognizers = Arc::new(FakeRecognizers::default());
        let keyboard = Arc::new(FakeKeyboard::default());
        let target = Arc::new(FakeTarget(Mutex::new(FakeTarget::app(42, "TextEdit"))));
        let detector = ContextDetectorHandle::start_with_source(
            target.clone(),
            AppRegistry::builtin().unwrap(),
            Duration::from_millis(20),
            Duration::from_secs(60),
        );
        let ports = SessionPorts {
            notifier: screen.clone(),
            settings: Arc::new(MemorySettings(config)),
            stores: stores.clone(),
            microphone: microphone.clone(),
            stt_providers: recognizers.clone(),
            llm_providers: Arc::new(NoPolish),
            input: keyboard.clone(),
            ask: Arc::new(NoAsk),
        };
        let session =
            PipelineHandle::with_ports(ports, StateCell::new(), reqwest::Client::new(), detector);
        Self {
            session,
            screen,
            stores,
            microphone,
            recognizers,
            keyboard,
            target,
        }
    }

    /// Phases the screen was told about, in order, since the last call.
    fn phases_shown(&self, notes: &[Note]) -> Vec<PipelineState> {
        notes
            .iter()
            .filter_map(|note| match note {
                Note::Phase(state) => Some(*state),
                _ => None,
            })
            .collect()
    }

    fn events_named(notes: &[Note], name: &str) -> Vec<serde_json::Value> {
        notes
            .iter()
            .filter_map(|note| match note {
                Note::Event(event, payload) if event == name => Some(payload.clone()),
                _ => None,
            })
            .collect()
    }

    async fn wait_until_shown(&self, state: PipelineState, seen: &mut Vec<Note>) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                seen.extend(self.screen.take());
                if seen.contains(&Note::Phase(state)) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{state:?} was never shown"));
    }
}

// ─── Scenarios ──────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn a_dictation_types_the_transcript_and_returns_to_idle() {
    let scenario = Scenario::new();
    scenario.recognizers.will_hear("회의는 3시입니다.");

    scenario.session.start().await.unwrap();
    assert_eq!(scenario.session.current_state(), PipelineState::Recording);
    scenario.session.stop().await.unwrap();

    assert_eq!(scenario.session.current_state(), PipelineState::Idle);
    assert_eq!(
        scenario.keyboard.typed(),
        [("회의는 3시입니다.".to_string(), InsertionStrategy::Auto)]
    );
    let notes = scenario.screen.take();
    assert_eq!(
        scenario.phases_shown(&notes),
        [
            PipelineState::Preparing,
            PipelineState::Recording,
            PipelineState::Transcribing,
            PipelineState::Outputting,
            PipelineState::Idle,
        ]
    );
    // Enter finishes a recording only while recording; it is let go before transcribing.
    let transcribing = notes
        .iter()
        .position(|note| *note == Note::Phase(PipelineState::Transcribing))
        .unwrap();
    assert_eq!(notes[transcribing - 1], Note::Enter(false));
    assert_eq!(scenario.stores.history_texts().await, ["회의는 3시입니다."]);
    assert_eq!(scenario.stores.saved_recordings(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_while_recording_types_nothing_and_keeps_no_audio() {
    let scenario = Scenario::new();
    scenario.recognizers.will_hear("보내지 마세요");

    scenario.session.start().await.unwrap();
    scenario.session.abort();
    scenario.session.stop().await.unwrap();

    assert_eq!(scenario.session.current_state(), PipelineState::Idle);
    assert!(scenario.keyboard.typed().is_empty());
    assert!(scenario.stores.history_texts().await.is_empty());
    assert_eq!(scenario.stores.saved_recordings(), 0);
    let notes = scenario.screen.take();
    assert_eq!(
        scenario.phases_shown(&notes),
        [
            PipelineState::Preparing,
            PipelineState::Recording,
            PipelineState::Idle,
        ]
    );
    assert_eq!(notes.last(), Some(&Note::Phase(PipelineState::Idle)));
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_while_the_microphone_warms_up_never_shows_recording() {
    let scenario = Arc::new(Scenario::new());
    scenario.recognizers.will_hear("들리면 안 됨");
    let warm_up = scenario.microphone.hold_ready();

    let starting = {
        let scenario = scenario.clone();
        tokio::spawn(async move { scenario.session.start().await })
    };
    let mut seen = Vec::new();
    scenario
        .wait_until_shown(PipelineState::Preparing, &mut seen)
        .await;
    scenario.session.abort();
    warm_up.notify_one();
    starting.await.unwrap().unwrap();
    seen.extend(scenario.screen.take());

    assert_eq!(scenario.session.current_state(), PipelineState::Idle);
    assert!(!scenario
        .phases_shown(&seen)
        .contains(&PipelineState::Recording));
    assert!(scenario.keyboard.typed().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_old_finish_stays_silent_after_cancel_and_a_new_recording() {
    let scenario = Arc::new(Scenario::new());
    let first_transcript = scenario.recognizers.will_hear_after_gate("첫 번째");
    scenario.recognizers.will_hear("두 번째");

    scenario.session.start().await.unwrap();
    let first_finish = {
        let scenario = scenario.clone();
        tokio::spawn(async move { scenario.session.stop().await })
    };
    let mut seen = Vec::new();
    scenario
        .wait_until_shown(PipelineState::Transcribing, &mut seen)
        .await;
    scenario.session.abort();
    first_finish.await.unwrap().unwrap();

    scenario.session.start().await.unwrap();
    first_transcript.notify_one();
    scenario.session.stop().await.unwrap();

    assert_eq!(
        scenario.keyboard.typed(),
        [("두 번째".to_string(), InsertionStrategy::Auto)]
    );
    assert_eq!(scenario.stores.history_texts().await, ["두 번째"]);
    assert_eq!(scenario.session.current_state(), PipelineState::Idle);
}

#[tokio::test(flavor = "multi_thread")]
async fn switching_apps_before_output_copies_instead_of_typing() {
    let scenario = Scenario::new();
    scenario.recognizers.will_hear("다른 창에 들어가면 안 됨");

    scenario.session.start().await.unwrap();
    scenario.target.switch_to(FakeTarget::app(77, "Slack"));
    scenario.session.stop().await.unwrap();

    assert_eq!(
        scenario.keyboard.typed(),
        [(
            "다른 창에 들어가면 안 됨".to_string(),
            InsertionStrategy::ClipboardCopyOnly
        )]
    );
    let notes = scenario.screen.take();
    let warnings = Scenario::events_named(&notes, "pipeline:warning");
    assert!(
        warnings
            .iter()
            .any(|warning| warning["code"] == "output_target_changed"),
        "{warnings:?}"
    );
    assert_eq!(scenario.session.current_state(), PipelineState::Idle);
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelling_while_idle_changes_nothing() {
    let scenario = Scenario::new();

    scenario.session.abort();

    assert_eq!(scenario.session.current_state(), PipelineState::Idle);
    assert!(scenario.screen.take().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn toggle_starts_when_idle_and_finishes_while_recording() {
    let scenario = Scenario::new();
    scenario.recognizers.will_hear("토글로 받아쓰기");

    scenario.session.toggle().await.unwrap();
    assert_eq!(scenario.session.current_state(), PipelineState::Recording);
    scenario.session.toggle().await.unwrap();

    assert_eq!(scenario.session.current_state(), PipelineState::Idle);
    assert_eq!(
        scenario.keyboard.typed(),
        [("토글로 받아쓰기".to_string(), InsertionStrategy::Auto)]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn toggle_while_preparing_finishes_once_the_microphone_is_ready() {
    let scenario = Arc::new(Scenario::new());
    scenario.recognizers.will_hear("준비 중에 끝내기");
    let warm_up = scenario.microphone.hold_ready();

    let starting = {
        let scenario = scenario.clone();
        tokio::spawn(async move { scenario.session.toggle().await })
    };
    let mut seen = Vec::new();
    scenario
        .wait_until_shown(PipelineState::Preparing, &mut seen)
        .await;
    let finishing = {
        let scenario = scenario.clone();
        tokio::spawn(async move { scenario.session.toggle().await })
    };
    warm_up.notify_one();
    starting.await.unwrap().unwrap();
    finishing.await.unwrap().unwrap();

    assert_eq!(scenario.session.current_state(), PipelineState::Idle);
    assert_eq!(
        scenario.keyboard.typed(),
        [("준비 중에 끝내기".to_string(), InsertionStrategy::Auto)]
    );
}
