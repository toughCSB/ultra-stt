use std::sync::Arc;

use super::phases::SessionNotifier;

/// What a dictation session and its background tasks use to talk to the screen.
#[derive(Clone)]
pub(crate) struct Events(Arc<dyn SessionNotifier>);

impl Events {
    pub(crate) fn new(notifier: Arc<dyn SessionNotifier>) -> Self {
        Self(notifier)
    }

    /// Sends `event` with `payload` serialized to JSON, exactly as the frontend expects it.
    pub(crate) fn send<T: serde::Serialize>(&self, event: &str, payload: T) {
        match serde_json::to_string(&payload) {
            Ok(json) => self.0.emit_json(event, json),
            Err(error) => tracing::warn!("Could not serialize {event} payload: {error}"),
        }
    }

    /// Tells the screen that the cloud sign-in expired, if `error` says so.
    pub(crate) fn cloud_session_invalid(&self, error: &crate::error::AppError) -> bool {
        crate::error::notify_cloud_session_invalid(error, |event| self.send(event, ()))
    }

    pub(crate) fn capture_recording_monitor(&self) {
        self.0.capture_recording_monitor();
    }

    pub(crate) fn capsule_has_focus(&self) -> bool {
        self.0.capsule_has_focus()
    }
}

#[cfg(test)]
mod tests {
    use super::super::phases::testing::{Note, RecordingNotifier};
    use super::*;

    #[test]
    fn payloads_reach_the_screen_as_json() {
        let notifier = Arc::new(RecordingNotifier::default());
        let events = Events::new(notifier.clone());

        events.send("stt:partial", "안녕");
        events.send("audio:volume", 0.5f32);
        events.send("pipeline:live_typing", true);

        assert_eq!(
            notifier.take(),
            [
                Note::Event("stt:partial".into(), serde_json::json!("안녕")),
                Note::Event("audio:volume".into(), serde_json::json!(0.5)),
                Note::Event("pipeline:live_typing".into(), serde_json::json!(true)),
            ]
        );
    }

    #[test]
    fn only_an_expired_cloud_sign_in_is_announced() {
        let notifier = Arc::new(RecordingNotifier::default());
        let events = Events::new(notifier.clone());

        assert!(!events.cloud_session_invalid(&crate::error::AppError::Network("down".into())));
        assert!(notifier.take().is_empty());

        assert!(events.cloud_session_invalid(&crate::error::AppError::CloudSessionInvalid));
        assert!(matches!(
            notifier.take().as_slice(),
            [Note::Event(_, serde_json::Value::Null)]
        ));
    }
}
