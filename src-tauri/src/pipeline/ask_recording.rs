//! Ask (질문) recordings. They use the same microphone, recognizer, time limit and session
//! as a dictation, but none of its extras (recovery audio, live typing, remote polish
//! options): what was heard goes back to the Ask flow, which answers it.

use super::*;
use crate::dictation_session::recording::{
    start_capture, CaptureStartError, RecordingLimit, RecordingWatch, StartedCapture,
};
use crate::recording_deadline::RecordingKind;
use crate::stt::SttProvider;

/// How long the recognizer gets to deliver the last words after an Ask recording stops.
const ASK_STT_FINALIZE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);

/// Where and about what the user asked.
pub(crate) struct AskContext {
    pub(crate) operation_id: String,
    pub(crate) recording_context: RecordingContext,
    pub(crate) selected_text: Option<String>,
}

/// A microphone and recognizer that are ready but not yet recording for the Ask.
pub(crate) struct PreparedAsk {
    session_id: u64,
    pub(crate) context: AskContext,
    capture: StartedCapture,
    provider: Box<dyn SttProvider>,
    limit: RecordingLimit,
    cloud_token: Option<String>,
}

impl PreparedAsk {
    /// Drops a capture that will not record.
    pub(crate) async fn discard(mut self) {
        self.capture.recorder.stop();
        let _ = self.provider.disconnect().await;
    }
}

/// The Ask recording in progress.
pub(super) struct AskTake {
    session_id: u64,
    recorder: Box<dyn Recorder>,
    context: AskContext,
    transcript: Arc<Mutex<String>>,
    error: Arc<Mutex<Option<String>>>,
    done: Arc<Notify>,
}

/// What an Ask recording heard.
pub(crate) struct AskHeard {
    pub(crate) context: AskContext,
    pub(crate) transcript: String,
    pub(crate) error: Option<String>,
    /// The recognizer did not finish in time; `transcript` holds what arrived.
    pub(crate) finalize_timed_out: bool,
}

impl PipelineHandle {
    /// Gets the microphone and recognizer ready for an Ask that already holds the session
    /// (`begin_ask`). Nothing records or shows until `begin_ask_recording`.
    pub(crate) async fn prepare_ask_recording(
        &self,
        include_selected_text: bool,
        operation_id: String,
    ) -> std::result::Result<PreparedAsk, String> {
        let session_id = self.session.begin();
        self.events.capture_recording_monitor();
        let config = self.settings.load_config().await;
        let recording_context = self
            .context_detector
            .snapshot_for_recording_enabled(config.context_adaptation_enabled);
        let selected_text = if include_selected_text && config.selected_text_enabled {
            tokio::task::block_in_place(|| self.input.selected_text())
        } else {
            None
        };
        let SttSetup {
            stt_config,
            custom_whisper,
        } = resolve_stt_setup(
            &config,
            self.settings.as_ref(),
            Some(operation_id.clone()),
            chrono::Utc::now().timestamp(),
        )
        .map_err(|error| match error {
            SttSetupError::SecretUnavailable(error) => error.to_string(),
            SttSetupError::MissingApiKey => {
                "STT API key is not configured. Please set it in Settings -> Speech Recognition."
                    .to_string()
            }
            SttSetupError::InvalidCustomWhisper(message) => message,
        })?;
        let cloud_token = (config.stt_provider == "cloud").then(|| stt_config.api_key.clone());
        if let Some(session_token) = cloud_token.clone() {
            stt::cloud::warm_managed_cloud_on_intent(self.shared_client.clone(), session_token);
        }
        let mut provider = self
            .stt_providers
            .create(&config, &stt_config, custom_whisper, None)
            .map_err(|error| error.to_string())?;
        let capture = start_capture(self.microphone.as_ref(), provider.as_mut(), &stt_config)
            .await
            .map_err(|error| match error {
                CaptureStartError::Microphone(error)
                | CaptureStartError::Startup(RecordingStartupError::Audio(error)) => {
                    microphone_error_message(&error.to_string())
                }
                CaptureStartError::Startup(RecordingStartupError::Stt(error)) => error.to_string(),
                CaptureStartError::Startup(RecordingStartupError::Timeout) => {
                    "Recording startup timed out after 30 seconds. Please try again.".to_string()
                }
            })?;
        let limit = RecordingLimit::for_recording(&config, provider.as_ref());
        Ok(PreparedAsk {
            session_id,
            context: AskContext {
                operation_id,
                recording_context,
                selected_text,
            },
            capture,
            provider,
            limit,
            cloud_token,
        })
    }

    /// Starts recording the question. False when the Ask was cancelled while preparing;
    /// the capture is then dropped.
    pub(crate) fn begin_ask_recording(&self, prepared: PreparedAsk) -> bool {
        if !self.session.is_live(prepared.session_id)
            || self.current_state() != PipelineState::AskRecording
        {
            tokio::spawn(prepared.discard());
            return false;
        }
        let PreparedAsk {
            session_id,
            context,
            capture,
            mut provider,
            limit,
            cloud_token,
        } = prepared;
        let StartedCapture {
            recorder,
            mut audio,
            ready_at,
        } = capture;
        let watch = RecordingWatch {
            events: self.events.clone(),
            kind: RecordingKind::Ask,
            session_id,
            ready_at,
            limit,
        };
        let transcript = Arc::new(Mutex::new(String::new()));
        let error = Arc::new(Mutex::new(None::<String>));
        let done = Arc::new(Notify::new());
        *self.ask_take.lock().unwrap_or_else(|e| e.into_inner()) = Some(AskTake {
            session_id,
            recorder,
            context,
            transcript: transcript.clone(),
            error: error.clone(),
            done: done.clone(),
        });
        self.show_ask_phase(PipelineState::AskRecording);
        self.events
            .send("recording:deadline", watch.deadline().event);
        if let Some(session_token) = cloud_token {
            let scope = self.session.recording_scope(session_id, &self.phases);
            watch.spawn_cloud_warmup(self.shared_client.clone(), session_token, move || {
                scope.is_active()
            });
        }

        // Forward audio to the recognizer and collect the question.
        let events = self.events.clone();
        let pipeline = self.clone();
        tokio::spawn(async move {
            let report = |message: String| {
                *error.lock().unwrap_or_else(|e| e.into_inner()) = Some(message.clone());
                if pipeline.is_recording_ask(session_id) {
                    pipeline.ask.recording_failed(message);
                }
            };
            loop {
                tokio::select! {
                    chunk = audio.recv() => match chunk {
                        Some(data) => {
                            if let Err(error) = provider.send_audio(&data).await {
                                events.cloud_session_invalid(&error);
                                report(error.to_string());
                                break;
                            }
                        }
                        None => {
                            match provider.disconnect().await {
                                Ok(Some(text)) => {
                                    events.send("ask:final", append_final_transcript(&transcript, &text));
                                }
                                Ok(None) => {}
                                Err(error) => {
                                    events.cloud_session_invalid(&error);
                                    report(error.to_string());
                                }
                            }
                            break;
                        }
                    },
                    event = provider.recv_transcript() => match event {
                        Ok(Some(TranscriptEvent::Partial { text })) => events.send("ask:partial", text),
                        Ok(Some(TranscriptEvent::Final { text, .. })) => {
                            events.send("ask:final", append_final_transcript(&transcript, &text));
                        }
                        Ok(Some(TranscriptEvent::Error { message })) => {
                            report(message);
                            break;
                        }
                        Err(error) => {
                            events.cloud_session_invalid(&error);
                            report(error.to_string());
                            break;
                        }
                        _ => {}
                    },
                }
            }
            done.notify_one();
        });

        let scope = self.session.recording_scope(session_id, &self.phases);
        let ask = self.ask.clone();
        watch.spawn_deadline(
            move || scope.is_active(),
            move || async move { ask.stop_requested() },
        );
        true
    }

    /// Stops the Ask recording and returns what the recognizer heard, waiting up to
    /// 12 seconds for the last words.
    pub(crate) async fn finish_ask_recording(&self) -> std::result::Result<AskHeard, String> {
        let mut take = self
            .ask_take
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .ok_or_else(|| "Ask dictation is not recording".to_string())?;
        take.recorder.stop();
        self.show_ask_phase(PipelineState::AskThinking);
        let finalize_timed_out = tokio::select! {
            _ = take.done.notified() => false,
            _ = tokio::time::sleep(ASK_STT_FINALIZE_TIMEOUT) => true,
        };
        let transcript = take
            .transcript
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let error = take.error.lock().unwrap_or_else(|e| e.into_inner()).clone();
        Ok(AskHeard {
            context: take.context,
            transcript,
            error,
            finalize_timed_out,
        })
    }

    /// Cancels the Ask recording, if any, and ends the session the Ask holds.
    pub(crate) fn cancel_ask_recording(&self) {
        if let Some(mut take) = self
            .ask_take
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            take.recorder.stop();
        }
        if self.current_state().is_ask() {
            self.session.end();
        }
    }

    /// Whether the Ask recording `session_id` is still recording.
    fn is_recording_ask(&self, session_id: u64) -> bool {
        self.ask_take
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|take| take.session_id == session_id)
    }
}

/// Appends a final recognizer segment to the question and returns the whole question so far.
fn append_final_transcript(transcript: &Arc<Mutex<String>>, text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return transcript.lock().unwrap_or_else(|e| e.into_inner()).clone();
    }

    let mut current = transcript.lock().unwrap_or_else(|e| e.into_inner());
    if !current.trim().is_empty() && !current.ends_with(' ') {
        current.push(' ');
    }
    current.push_str(text);
    current.trim().to_string()
}

/// Turns a microphone failure into a short message the user can act on.
pub(crate) fn microphone_error_message(message: &str) -> String {
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("permission")
        || normalized.contains("access denied")
        || normalized.contains("not authorized")
    {
        return "Microphone permission is required.".to_string();
    }
    "Microphone unavailable. Check your input device.".to_string()
}
