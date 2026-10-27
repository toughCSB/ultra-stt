use async_trait::async_trait;

use crate::llm::LlmProvider;
use crate::output::clipboard::ClipboardOutputOptions;
use crate::output::windows_sendinput::WindowsSendInputOptions;
use crate::output::{InsertResult, InsertionStrategy, OutputOutcome};

/// Typing into the user's app: the final text, live chunks, and reading the selection.
#[async_trait]
pub(crate) trait TextInput: Send + Sync {
    /// Inserts `text` at the cursor with `strategy`, falling back as the strategy allows.
    async fn insert(
        &self,
        text: &str,
        strategy: InsertionStrategy,
        clipboard: ClipboardOutputOptions,
        sendinput: WindowsSendInputOptions,
    ) -> Result<OutputOutcome, String>;

    /// Types one chunk directly (live typing and streamed polish).
    async fn insert_chunk(
        &self,
        chunk: &str,
        strategy: InsertionStrategy,
        sendinput: WindowsSendInputOptions,
    ) -> Result<InsertResult, String>;

    /// The text selected in the user's app, if any. Blocks while it copies the selection.
    fn selected_text(&self) -> Option<String>;
}

/// The system's keyboard, clipboard and selection.
pub(crate) struct SystemTextInput {
    pub(crate) app_handle: tauri::AppHandle,
}

#[async_trait]
impl TextInput for SystemTextInput {
    async fn insert(
        &self,
        text: &str,
        strategy: InsertionStrategy,
        clipboard: ClipboardOutputOptions,
        sendinput: WindowsSendInputOptions,
    ) -> Result<OutputOutcome, String> {
        crate::output::output_with_strategy(&self.app_handle, text, strategy, clipboard, sendinput)
            .await
    }

    async fn insert_chunk(
        &self,
        chunk: &str,
        strategy: InsertionStrategy,
        sendinput: WindowsSendInputOptions,
    ) -> Result<InsertResult, String> {
        crate::output::output_stream_chunk(&self.app_handle, chunk, strategy, sendinput).await
    }

    fn selected_text(&self) -> Option<String> {
        crate::selection::capture_selected_text()
    }
}

/// Builds the AI polish provider named in the settings.
pub(crate) trait LlmProviders: Send + Sync {
    fn create(&self, provider_name: &str) -> Box<dyn LlmProvider>;
}

pub(crate) struct AppLlmProviders {
    pub(crate) client: reqwest::Client,
}

impl LlmProviders for AppLlmProviders {
    fn create(&self, provider_name: &str) -> Box<dyn LlmProvider> {
        crate::llm::create_provider(provider_name, Some(self.client.clone()))
    }
}
