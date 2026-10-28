//! Parts every recording shares, whether it is a dictation or an Ask: starting the microphone
//! together with the recognizer, the recording time limit, and keeping the cloud warm.

use std::future::Future;

use tokio::sync::mpsc;

use super::capture::{Microphone, Recorder};
use super::events::Events;
use crate::audio::RecordingStartupError;
use crate::error::AppError;
use crate::recording_deadline::{
    drive_recording_deadline, CaptureReadyAt, RecordingDeadline, RecordingDeadlineSignal,
    RecordingKind,
};
use crate::storage::AppConfig;
use crate::stt::{SttConfig, SttProvider};

/// A running capture: the recorder, its audio chunks, and when it really started.
pub(crate) struct StartedCapture {
    pub(crate) recorder: Box<dyn Recorder>,
    pub(crate) audio: mpsc::Receiver<Vec<u8>>,
    pub(crate) ready_at: CaptureReadyAt,
}

pub(crate) enum CaptureStartError {
    /// The microphone could not be opened.
    Microphone(anyhow::Error),
    /// The microphone or the recognizer did not get ready.
    Startup(RecordingStartupError<anyhow::Error, AppError>),
}

/// Opens the microphone, then waits for it and the recognizer together, so speech captured
/// while a network recognizer connects stays queued instead of being clipped.
pub(crate) async fn start_capture(
    microphone: &dyn Microphone,
    provider: &mut dyn SttProvider,
    stt_config: &SttConfig,
) -> Result<StartedCapture, CaptureStartError> {
    let (mut recorder, audio) = microphone.start().map_err(CaptureStartError::Microphone)?;
    match crate::audio::await_recording_startup(
        recorder.wait_until_ready(),
        provider.connect(stt_config),
    )
    .await
    {
        Ok(ready_at) => Ok(StartedCapture {
            recorder,
            audio,
            ready_at,
        }),
        Err(error) => {
            recorder.stop();
            Err(CaptureStartError::Startup(error))
        }
    }
}

/// How long one recording may run and why.
#[derive(Clone)]
pub(crate) struct RecordingLimit {
    pub(crate) max_seconds: u32,
    pub(crate) provider_id: String,
    pub(crate) explanation_key: String,
}

impl RecordingLimit {
    /// The configured limit, shortened when the recognizer itself allows less.
    pub(crate) fn for_recording(config: &AppConfig, provider: &dyn SttProvider) -> Self {
        let resolved = crate::stt::capabilities::resolve_recording_limit(
            config,
            None,
            chrono::Utc::now().timestamp(),
        );
        let max_seconds = provider
            .recording_limit_override_seconds()
            .map_or(resolved.effective_max_seconds, |override_seconds| {
                resolved.effective_max_seconds.min(override_seconds)
            });
        let explanation_key = provider
            .recording_limit_override_explanation_key()
            .unwrap_or(&resolved.capability.explanation_key)
            .to_string();
        Self {
            max_seconds,
            provider_id: resolved.capability.provider_id,
            explanation_key,
        }
    }
}

/// Watches one recording from the moment the microphone is ready.
pub(crate) struct RecordingWatch {
    pub(crate) events: Events,
    pub(crate) kind: RecordingKind,
    pub(crate) session_id: u64,
    pub(crate) ready_at: CaptureReadyAt,
    pub(crate) limit: RecordingLimit,
}

impl RecordingWatch {
    /// When the recording will stop; the screen counts down to it.
    pub(crate) fn deadline(&self) -> RecordingDeadline {
        RecordingDeadline::new(
            self.session_id,
            self.kind,
            self.ready_at,
            self.limit.max_seconds,
        )
    }

    /// Warns before the limit (60 and 10 seconds for long limits), then calls `on_reached`,
    /// which finishes the recording with what was heard so far. Stops early once
    /// `is_active` turns false.
    pub(crate) fn spawn_deadline<OnReached, Done>(
        &self,
        is_active: impl Fn() -> bool + Send + 'static,
        on_reached: OnReached,
    ) where
        OnReached: FnOnce() -> Done + Send + 'static,
        Done: Future<Output = ()> + Send + 'static,
    {
        let deadline = self.deadline();
        let events = self.events.clone();
        let kind = self.kind;
        let session_id = self.session_id;
        let limit = self.limit.clone();
        tokio::spawn(async move {
            let reached = drive_recording_deadline(deadline, is_active, |signal| match signal {
                RecordingDeadlineSignal::Warning { seconds_remaining } => events.send(
                    "recording:deadline-warning",
                    serde_json::json!({
                        "sessionId": session_id,
                        "recordingKind": kind,
                        "secondsRemaining": seconds_remaining,
                        "effectiveMaxSeconds": limit.max_seconds,
                        "providerId": limit.provider_id.as_str(),
                        "explanationKey": limit.explanation_key.as_str(),
                    }),
                ),
                RecordingDeadlineSignal::Reached => events.send(
                    "recording:deadline-reached",
                    serde_json::json!({
                        "sessionId": session_id,
                        "recordingKind": kind,
                        "effectiveMaxSeconds": limit.max_seconds,
                        "providerId": limit.provider_id.as_str(),
                        "explanationKey": limit.explanation_key.as_str(),
                    }),
                ),
            })
            .await;
            if reached {
                on_reached().await;
            }
        });
    }

    /// Wakes the managed cloud recognizer at 4 and 8 minutes of a long recording.
    pub(crate) fn spawn_cloud_warmup(
        &self,
        client: reqwest::Client,
        session_token: String,
        is_active: impl Fn() -> bool + Send + 'static,
    ) {
        let max_seconds = self.limit.max_seconds;
        let ready_at = self.ready_at;
        tokio::spawn(async move {
            for elapsed_seconds in [240u64, 480] {
                if elapsed_seconds >= u64::from(max_seconds) {
                    break;
                }
                tokio::time::sleep_until(tokio::time::Instant::from_std(
                    ready_at.monotonic + std::time::Duration::from_secs(elapsed_seconds),
                ))
                .await;
                if !is_active() {
                    break;
                }
                crate::stt::cloud::warm_managed_cloud_on_intent(
                    client.clone(),
                    session_token.clone(),
                );
            }
        });
    }
}
