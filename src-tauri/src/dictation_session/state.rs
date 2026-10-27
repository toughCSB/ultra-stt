use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

/// Phase of a dictation session. The frontend receives it as a snake_case string in `pipeline:state`.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Idle,
    Preparing,
    Recording,
    Transcribing,
    Polishing,
    Outputting,
    AskRecording,
    AskThinking,
}

impl PipelineState {
    fn as_u8(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Preparing => 1,
            Self::Recording => 2,
            Self::Transcribing => 3,
            Self::Polishing => 4,
            Self::Outputting => 5,
            Self::AskRecording => 6,
            Self::AskThinking => 7,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Preparing,
            2 => Self::Recording,
            3 => Self::Transcribing,
            4 => Self::Polishing,
            5 => Self::Outputting,
            6 => Self::AskRecording,
            7 => Self::AskThinking,
            _ => Self::Idle,
        }
    }

    /// Escape (cancel) is held whenever a session is running.
    pub(crate) fn holds_escape(self) -> bool {
        self != Self::Idle
    }

    /// Enter (finish recording) is held only while recording, so it never swallows Enter in other apps.
    pub(crate) fn holds_enter(self) -> bool {
        self == Self::Recording
    }
}

/// Shared slot holding the current phase. Clones see the same slot.
/// Start and finish use `claim` so that only one caller gets through.
#[derive(Clone)]
pub(crate) struct StateCell(Arc<AtomicU8>);

impl StateCell {
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicU8::new(PipelineState::Idle.as_u8())))
    }

    pub(crate) fn get(&self) -> PipelineState {
        PipelineState::from_u8(self.0.load(Ordering::SeqCst))
    }

    pub(crate) fn set(&self, state: PipelineState) {
        self.0.store(state.as_u8(), Ordering::SeqCst);
    }

    /// Moves to `to` only if the current phase is `from`; returns whether it did.
    pub(crate) fn claim(&self, from: PipelineState, to: PipelineState) -> bool {
        self.0
            .compare_exchange(from.as_u8(), to.as_u8(), Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [PipelineState; 8] = [
        PipelineState::Idle,
        PipelineState::Preparing,
        PipelineState::Recording,
        PipelineState::Transcribing,
        PipelineState::Polishing,
        PipelineState::Outputting,
        PipelineState::AskRecording,
        PipelineState::AskThinking,
    ];

    #[test]
    fn every_state_has_a_frontend_name_and_survives_the_cell() {
        let names = [
            "idle",
            "preparing",
            "recording",
            "transcribing",
            "polishing",
            "outputting",
            "ask_recording",
            "ask_thinking",
        ];
        let cell = StateCell::new();
        for (state, name) in ALL.into_iter().zip(names) {
            assert_eq!(
                serde_json::to_value(state).unwrap(),
                serde_json::json!(name)
            );
            cell.set(state);
            assert_eq!(cell.get(), state);
        }
    }

    #[test]
    fn recording_keys_are_held_per_state() {
        // (phase, holds Escape, holds Enter)
        let table = [
            (PipelineState::Idle, false, false),
            (PipelineState::Preparing, true, false),
            (PipelineState::Recording, true, true),
            (PipelineState::Transcribing, true, false),
            (PipelineState::Polishing, true, false),
            (PipelineState::Outputting, true, false),
            (PipelineState::AskRecording, true, false),
            (PipelineState::AskThinking, true, false),
        ];
        for (state, escape, enter) in table {
            assert_eq!(state.holds_escape(), escape, "{state:?} Escape");
            assert_eq!(state.holds_enter(), enter, "{state:?} Enter");
        }
    }

    #[test]
    fn claim_lets_only_one_caller_through() {
        let cell = StateCell::new();
        let shared = cell.clone();
        assert!(cell.claim(PipelineState::Idle, PipelineState::Preparing));
        assert!(!shared.claim(PipelineState::Idle, PipelineState::Preparing));
        assert_eq!(shared.get(), PipelineState::Preparing);
        assert!(!cell.claim(PipelineState::Recording, PipelineState::Transcribing));
        assert_eq!(cell.get(), PipelineState::Preparing);
    }
}
