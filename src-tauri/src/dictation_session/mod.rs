//! Dictation session (받아쓰기 세션): one dictation from the start of recording until the
//! result is inserted, cancelled or fails. Terms follow `GLOSSARY.md` at the repository root.

mod ask;
mod capture;
mod events;
mod input;
mod phases;
pub(crate) mod recording;
#[cfg(test)]
mod scenarios;
mod session;
mod state;
mod storage;

use std::sync::Arc;

pub(crate) use ask::AskWindows;
pub(crate) use capture::{AppSttProviders, Microphone, Recorder, SttProviders, SystemMicrophone};
pub(crate) use events::Events;
pub(crate) use input::{AppLlmProviders, LlmProviders, SystemTextInput, TextInput};
pub(crate) use phases::{Phases, SessionNotifier};
pub(crate) use session::SessionClock;
pub use state::PipelineState;
pub(crate) use state::StateCell;
pub(crate) use storage::{correction_rules_for_polish, SessionSettings, SessionStores};

/// Everything a dictation session needs from outside itself. The app passes Tauri and
/// system adapters; tests pass fakes.
pub(crate) struct SessionPorts {
    pub(crate) notifier: Arc<dyn SessionNotifier>,
    pub(crate) settings: Arc<dyn SessionSettings>,
    pub(crate) stores: Arc<dyn SessionStores>,
    pub(crate) microphone: Arc<dyn Microphone>,
    pub(crate) stt_providers: Arc<dyn SttProviders>,
    pub(crate) llm_providers: Arc<dyn LlmProviders>,
    pub(crate) input: Arc<dyn TextInput>,
    pub(crate) ask: Arc<dyn AskWindows>,
}
