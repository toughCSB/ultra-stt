use std::sync::Arc;

use super::state::{PipelineState, StateCell};

/// Screen-side effects of a dictation session: the capsule, the tray and the recording keys.
/// The app uses a Tauri adapter; tests use a recorder.
pub(crate) trait SessionNotifier: Send + Sync {
    /// Hold or release Escape (cancel).
    fn hold_escape(&self, hold: bool);
    /// Hold or release Enter (finish recording).
    fn hold_enter(&self, hold: bool);
    /// Tell the capsule and the tray that a phase was entered.
    fn phase_entered(&self, state: PipelineState);
    /// Send a session event (`pipeline:error`, `stt:partial`, ...) with a JSON payload.
    fn emit_json(&self, event: &str, payload: String);
    /// Remember the monitor the user is working on, so the capsule appears there.
    fn capture_recording_monitor(&self);
    /// Whether the capsule took keyboard focus (the user clicked it during the session).
    fn capsule_has_focus(&self) -> bool;
}

/// The one place where a dictation session changes phase. Every change updates the
/// recording keys first and then tells the screen, so they never disagree with the phase.
#[derive(Clone)]
pub(crate) struct Phases {
    cell: StateCell,
    notifier: Arc<dyn SessionNotifier>,
}

impl Phases {
    pub(crate) fn new(cell: StateCell, notifier: Arc<dyn SessionNotifier>) -> Self {
        Self { cell, notifier }
    }

    pub(crate) fn get(&self) -> PipelineState {
        self.cell.get()
    }

    /// Reserves `from` → `to` without telling anyone yet; follow with `enter(to)`.
    /// Start uses this so it can prepare (pick the capsule monitor) before the capsule shows.
    pub(crate) fn claim(&self, from: PipelineState, to: PipelineState) -> bool {
        self.cell.claim(from, to)
    }

    /// Enters `state` and announces it.
    pub(crate) fn enter(&self, state: PipelineState) {
        self.cell.set(state);
        self.announce(state);
    }

    /// Moves `from` → `to` and announces it, only if the phase is still `from`.
    pub(crate) fn advance(&self, from: PipelineState, to: PipelineState) -> bool {
        let moved = self.cell.claim(from, to);
        if moved {
            self.announce(to);
        }
        moved
    }

    fn announce(&self, state: PipelineState) {
        self.notifier.hold_escape(state.holds_escape());
        self.notifier.hold_enter(state.holds_enter());
        self.notifier.phase_entered(state);
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use std::sync::Mutex;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub(crate) enum Note {
        Escape(bool),
        Enter(bool),
        Phase(PipelineState),
        Event(String, serde_json::Value),
        CaptureMonitor,
    }

    /// Records what a session told the screen, in order.
    #[derive(Default)]
    pub(crate) struct RecordingNotifier(Mutex<Vec<Note>>);

    impl RecordingNotifier {
        pub(crate) fn take(&self) -> Vec<Note> {
            std::mem::take(&mut *self.0.lock().unwrap())
        }
    }

    impl SessionNotifier for RecordingNotifier {
        fn hold_escape(&self, hold: bool) {
            self.0.lock().unwrap().push(Note::Escape(hold));
        }
        fn hold_enter(&self, hold: bool) {
            self.0.lock().unwrap().push(Note::Enter(hold));
        }
        fn phase_entered(&self, state: PipelineState) {
            self.0.lock().unwrap().push(Note::Phase(state));
        }
        fn emit_json(&self, event: &str, payload: String) {
            let payload = serde_json::from_str(&payload).expect("events carry JSON");
            self.0
                .lock()
                .unwrap()
                .push(Note::Event(event.to_string(), payload));
        }
        fn capture_recording_monitor(&self) {
            self.0.lock().unwrap().push(Note::CaptureMonitor);
        }
        fn capsule_has_focus(&self) -> bool {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{Note, RecordingNotifier};
    use super::*;

    fn phases() -> (Phases, Arc<RecordingNotifier>) {
        let notifier = Arc::new(RecordingNotifier::default());
        (Phases::new(StateCell::new(), notifier.clone()), notifier)
    }

    #[test]
    fn finishing_a_recording_releases_enter_before_transcribing_is_shown() {
        let (phases, notes) = phases();
        assert!(phases.claim(PipelineState::Idle, PipelineState::Preparing));
        phases.enter(PipelineState::Preparing);
        phases.enter(PipelineState::Recording);
        notes.take();

        assert!(phases.advance(PipelineState::Recording, PipelineState::Transcribing));

        assert_eq!(
            notes.take(),
            [
                Note::Escape(true),
                Note::Enter(false),
                Note::Phase(PipelineState::Transcribing),
            ]
        );
    }

    #[test]
    fn a_second_finish_changes_nothing_and_tells_no_one() {
        let (phases, notes) = phases();
        phases.enter(PipelineState::Recording);
        assert!(phases.advance(PipelineState::Recording, PipelineState::Transcribing));
        notes.take();

        assert!(!phases.advance(PipelineState::Recording, PipelineState::Transcribing));

        assert_eq!(phases.get(), PipelineState::Transcribing);
        assert!(notes.take().is_empty());
    }

    #[test]
    fn claiming_start_is_silent_until_preparing_is_entered() {
        let (phases, notes) = phases();
        assert!(phases.claim(PipelineState::Idle, PipelineState::Preparing));
        assert!(notes.take().is_empty());

        phases.enter(PipelineState::Preparing);

        assert_eq!(
            notes.take(),
            [
                Note::Escape(true),
                Note::Enter(false),
                Note::Phase(PipelineState::Preparing),
            ]
        );
    }

    #[test]
    fn every_phase_announces_keys_matching_the_phase() {
        let (phases, notes) = phases();
        for state in [
            PipelineState::Preparing,
            PipelineState::Recording,
            PipelineState::Transcribing,
            PipelineState::Polishing,
            PipelineState::Outputting,
            PipelineState::Idle,
        ] {
            phases.enter(state);
            assert_eq!(
                notes.take(),
                [
                    Note::Escape(state.holds_escape()),
                    Note::Enter(state.holds_enter()),
                    Note::Phase(state),
                ],
                "{state:?}"
            );
            assert_eq!(phases.get(), state);
        }
    }
}
