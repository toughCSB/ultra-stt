use crate::voice_intent::VoiceIntentKind;

/// The Ask windows, and the Ask recording a dictation session must give way to.
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
}
