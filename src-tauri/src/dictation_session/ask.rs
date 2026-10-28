use crate::voice_intent::VoiceIntentKind;

/// The Ask flow around an Ask recording: its windows, and the Ask state that decides
/// whether a dictation must give way.
pub(crate) trait AskWindows: Send + Sync {
    /// An Ask recording or answer is in progress; Escape cancels it instead of dictation.
    fn busy(&self) -> bool;
    /// Cancels the Ask in progress.
    fn abort(&self);
    /// Hides the answer window so the user's app can take focus back.
    fn hide_answer(&self);
    fn show_answer(
        &self,
        question: String,
        answer: String,
        intent: VoiceIntentKind,
        used_selected_text: bool,
        selected_text_truncated: bool,
    ) -> Result<(), String>;
    fn show_error(&self, message: String) -> Result<(), String>;
    /// The Ask recording reached its time limit: finish it and answer what was heard.
    fn stop_requested(&self);
    /// The recognizer failed while the Ask was still recording.
    fn recording_failed(&self, message: String);
}
