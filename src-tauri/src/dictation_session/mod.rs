//! Dictation session (받아쓰기 세션): one dictation from the start of recording until the
//! result is inserted, cancelled or fails. Terms follow `GLOSSARY.md` at the repository root.

mod state;

pub use state::PipelineState;
pub(crate) use state::StateCell;
