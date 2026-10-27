//! Dictation session (받아쓰기 세션): one dictation from the start of recording until the
//! result is inserted, cancelled or fails. Terms follow `GLOSSARY.md` at the repository root.

mod capture;
mod events;
mod input;
mod phases;
mod session;
mod state;
mod storage;

pub(crate) use capture::{AppSttProviders, Microphone, Recorder, SttProviders, SystemMicrophone};
pub(crate) use events::Events;
pub(crate) use input::{AppLlmProviders, LlmProviders, SystemTextInput, TextInput};
pub(crate) use phases::{Phases, SessionNotifier};
pub(crate) use session::SessionClock;
pub use state::PipelineState;
pub(crate) use state::StateCell;
pub(crate) use storage::{correction_rules_for_polish, SessionSettings, SessionStores};
