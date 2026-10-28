//! The global keys a dictation session holds while it runs: Escape (cancel) and Enter
//! (finish recording). What the session wants and what is really registered with the OS
//! are tracked apart, so removing every shortcut (re-registering the configured ones, or
//! pausing them for key capture) can never leave a wanted key silently missing.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecordingKey {
    Escape,
    Enter,
}

/// What the caller must do with the OS for one key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyChange {
    Register(RecordingKey),
    Unregister(RecordingKey),
}

#[derive(Clone, Copy, Debug, Default)]
struct KeyState {
    wanted: bool,
    registered: bool,
}

#[derive(Debug, Default)]
pub(crate) struct RecordingKeys {
    escape: KeyState,
    enter: KeyState,
    /// Shortcuts are paused while the settings capture a new hotkey.
    paused: bool,
}

impl RecordingKeys {
    fn state(&mut self, key: RecordingKey) -> &mut KeyState {
        match key {
            RecordingKey::Escape => &mut self.escape,
            RecordingKey::Enter => &mut self.enter,
        }
    }

    /// The session wants `key` held or released.
    pub(crate) fn hold(&mut self, key: RecordingKey, hold: bool) -> Option<KeyChange> {
        let paused = self.paused;
        let state = self.state(key);
        state.wanted = hold;
        match (hold, state.registered) {
            (true, false) if !paused => Some(KeyChange::Register(key)),
            (false, true) => Some(KeyChange::Unregister(key)),
            _ => None,
        }
    }

    /// Reports how a `Register` went. A key that is not ours (already taken) stays unregistered.
    pub(crate) fn registered(&mut self, key: RecordingKey, succeeded: bool) {
        self.state(key).registered = succeeded;
    }

    /// Reports that an `Unregister` was carried out.
    pub(crate) fn unregistered(&mut self, key: RecordingKey) {
        self.state(key).registered = false;
    }

    /// Every global shortcut was removed. Wanted keys come back at once unless shortcuts
    /// are now paused; then they come back when shortcuts are registered again.
    pub(crate) fn all_removed(&mut self, paused: bool) -> Vec<KeyChange> {
        self.paused = paused;
        let mut changes = Vec::new();
        for key in [RecordingKey::Escape, RecordingKey::Enter] {
            let state = self.state(key);
            state.registered = false;
            if state.wanted && !paused {
                changes.push(KeyChange::Register(key));
            }
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use RecordingKey::{Enter, Escape};

    #[test]
    fn a_held_key_is_registered_once_and_released_once() {
        let mut keys = RecordingKeys::default();
        assert_eq!(keys.hold(Escape, true), Some(KeyChange::Register(Escape)));
        keys.registered(Escape, true);
        assert_eq!(keys.hold(Escape, true), None);
        assert_eq!(
            keys.hold(Escape, false),
            Some(KeyChange::Unregister(Escape))
        );
        keys.unregistered(Escape);
        assert_eq!(keys.hold(Escape, false), None);
    }

    #[test]
    fn removing_all_shortcuts_mid_session_brings_held_keys_back() {
        let mut keys = RecordingKeys::default();
        keys.hold(Escape, true);
        keys.registered(Escape, true);
        keys.hold(Enter, true);
        keys.registered(Enter, true);

        // Saving settings re-registers the configured hotkeys after removing everything.
        assert_eq!(
            keys.all_removed(false),
            [KeyChange::Register(Escape), KeyChange::Register(Enter)]
        );
        keys.registered(Escape, true);
        keys.registered(Enter, true);

        // Recording finished: Enter goes, Escape stays for the transcription.
        assert_eq!(keys.hold(Enter, false), Some(KeyChange::Unregister(Enter)));
    }

    #[test]
    fn keys_not_held_stay_unregistered_when_shortcuts_are_replaced() {
        let mut keys = RecordingKeys::default();
        assert!(keys.all_removed(false).is_empty());
        keys.hold(Escape, true);
        keys.registered(Escape, true);
        keys.hold(Escape, false);
        keys.unregistered(Escape);
        assert!(keys.all_removed(false).is_empty());
    }

    #[test]
    fn paused_shortcuts_hold_nothing_until_registered_again() {
        let mut keys = RecordingKeys::default();
        keys.hold(Escape, true);
        keys.registered(Escape, true);

        assert!(keys.all_removed(true).is_empty());
        assert_eq!(keys.hold(Enter, true), None);

        assert_eq!(
            keys.all_removed(false),
            [KeyChange::Register(Escape), KeyChange::Register(Enter)]
        );
    }

    #[test]
    fn a_key_taken_by_something_else_is_not_released_by_the_session() {
        let mut keys = RecordingKeys::default();
        assert_eq!(keys.hold(Escape, true), Some(KeyChange::Register(Escape)));
        keys.registered(Escape, false);
        assert_eq!(keys.hold(Escape, false), None);
    }
}
