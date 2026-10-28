use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::watch;

use super::phases::Phases;
use super::state::PipelineState;

/// Identifies the current dictation session. Starting a session, cancelling it or starting an
/// Ask draft moves to a new id; every task that still holds an older id must stand down.
#[derive(Clone)]
pub(crate) struct SessionClock {
    current: Arc<AtomicU64>,
    changed: watch::Sender<u64>,
}

impl SessionClock {
    pub(crate) fn new() -> Self {
        Self {
            current: Arc::new(AtomicU64::new(0)),
            changed: watch::channel(0).0,
        }
    }

    /// Ends the current session and returns the id of the one that begins now.
    pub(crate) fn begin(&self) -> u64 {
        let id = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.changed.send_replace(id);
        id
    }

    /// Ends the current session without starting work under the new id (cancel).
    pub(crate) fn end(&self) {
        self.begin();
    }

    pub(crate) fn current(&self) -> u64 {
        self.current.load(Ordering::SeqCst)
    }

    pub(crate) fn is_live(&self, id: u64) -> bool {
        self.current() == id
    }

    /// Notifies on every session change after this call.
    pub(crate) fn watch(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    /// The recording part of session `id`. It ends when that session ends or leaves Recording.
    pub(crate) fn recording_scope(&self, id: u64, phases: &Phases) -> RecordingScope {
        RecordingScope {
            clock: self.clone(),
            phases: phases.clone(),
            id,
        }
    }
}

/// Lifetime of work that only makes sense while one session is recording, as a dictation
/// or an Ask (the recording time limit and the cloud warm-up loop).
#[derive(Clone)]
pub(crate) struct RecordingScope {
    clock: SessionClock,
    phases: Phases,
    id: u64,
}

impl RecordingScope {
    pub(crate) fn is_active(&self) -> bool {
        self.clock.is_live(self.id)
            && matches!(
                self.phases.get(),
                PipelineState::Recording | PipelineState::AskRecording
            )
    }
}

#[cfg(test)]
mod tests {
    use super::super::phases::testing::RecordingNotifier;
    use super::super::state::StateCell;
    use super::*;

    #[test]
    fn only_the_newest_session_is_live() {
        let clock = SessionClock::new();
        let first = clock.begin();
        assert!(clock.is_live(first));

        clock.end();
        assert!(!clock.is_live(first));

        let second = clock.begin();
        assert!(clock.is_live(second));
        assert!(!clock.is_live(first));
    }

    #[tokio::test]
    async fn an_old_stop_wakes_and_stays_cancelled_when_a_new_session_begins() {
        let clock = SessionClock::new();
        let old = clock.begin();
        let mut changed = clock.watch();
        let old_stop = async {
            changed.wait_for(|id| *id != old).await.unwrap();
            assert!(!clock.is_live(old));
        };
        let cancel_then_restart = async {
            clock.end();
            clock.begin();
        };
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            tokio::join!(old_stop, cancel_then_restart);
        })
        .await
        .unwrap();
        assert!(!clock.is_live(old));
    }

    #[test]
    fn recording_scope_ends_when_recording_finishes_or_the_session_ends() {
        let clock = SessionClock::new();
        let phases = Phases::new(StateCell::new(), Arc::new(RecordingNotifier::default()));
        let id = clock.begin();
        let scope = clock.recording_scope(id, &phases);

        phases.enter(PipelineState::Preparing);
        assert!(!scope.is_active());
        phases.enter(PipelineState::Recording);
        assert!(scope.is_active());
        phases.enter(PipelineState::Transcribing);
        assert!(!scope.is_active());

        phases.enter(PipelineState::Recording);
        clock.end();
        assert!(!scope.is_active());

        let ask = clock.begin();
        let ask_scope = clock.recording_scope(ask, &phases);
        phases.enter(PipelineState::AskRecording);
        assert!(ask_scope.is_active());
        phases.enter(PipelineState::AskThinking);
        assert!(!ask_scope.is_active());
    }
}
