//! Dictation session (받아쓰기 세션): one dictation from the start of recording until the
//! result is inserted, cancelled or fails. Terms follow `GLOSSARY.md` at the repository root.

mod phases;
mod session;
mod state;

pub(crate) use phases::{Phases, SessionNotifier};
pub(crate) use session::SessionClock;
pub use state::PipelineState;
pub(crate) use state::StateCell;
