use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::audio::{AudioCaptureHandle, AudioConfig};
use crate::error::AppError;
use crate::recording_deadline::CaptureReadyAt;
use crate::storage::AppConfig;
use crate::stt::whisper_compat::WhisperCompatConfig;
use crate::stt::{SttConfig, SttProvider};

/// One running microphone capture. Audio chunks arrive on the receiver returned with it.
#[async_trait]
pub(crate) trait Recorder: Send {
    /// Resolves once the microphone is really delivering audio.
    async fn wait_until_ready(&mut self) -> anyhow::Result<CaptureReadyAt>;
    fn stop(&mut self);
    fn volume(&self) -> f32;
    /// Set when the device fails after recording began.
    fn runtime_error(&self) -> Option<String>;
}

/// A started capture and the audio chunks it delivers.
pub(crate) type Capture = (Box<dyn Recorder>, mpsc::Receiver<Vec<u8>>);

/// Starts microphone captures.
pub(crate) trait Microphone: Send + Sync {
    fn start(&self) -> anyhow::Result<Capture>;
}

/// The system microphone (CPAL).
pub(crate) struct SystemMicrophone;

impl Microphone for SystemMicrophone {
    fn start(&self) -> anyhow::Result<Capture> {
        let (handle, audio) = AudioCaptureHandle::start(AudioConfig::default())?;
        Ok((Box::new(handle), audio))
    }
}

#[async_trait]
impl Recorder for AudioCaptureHandle {
    async fn wait_until_ready(&mut self) -> anyhow::Result<CaptureReadyAt> {
        AudioCaptureHandle::wait_until_ready(self).await
    }

    fn stop(&mut self) {
        AudioCaptureHandle::stop(self);
    }

    fn volume(&self) -> f32 {
        self.get_volume()
    }

    fn runtime_error(&self) -> Option<String> {
        AudioCaptureHandle::runtime_error(self)
    }
}

/// Dictionary words and (pattern, replacement) corrections for live typing.
pub(crate) type LiveHints = (Vec<String>, Vec<(String, String)>);

/// Builds the speech recognizer for one recording from the settings.
pub(crate) trait SttProviders: Send + Sync {
    /// `live_hints` is set for live typing, which recognizes while the user speaks.
    fn create(
        &self,
        config: &AppConfig,
        stt_config: &SttConfig,
        custom_whisper: Option<WhisperCompatConfig>,
        live_hints: Option<LiveHints>,
    ) -> Result<Box<dyn SttProvider>, AppError>;
}

/// The app's recognizers: local or remote live passes for live typing, otherwise the
/// configured provider.
pub(crate) struct AppSttProviders {
    pub(crate) client: reqwest::Client,
}

impl SttProviders for AppSttProviders {
    fn create(
        &self,
        config: &AppConfig,
        stt_config: &SttConfig,
        custom_whisper: Option<WhisperCompatConfig>,
        live_hints: Option<LiveHints>,
    ) -> Result<Box<dyn SttProvider>, AppError> {
        use crate::stt::{self, ultra_local};

        let Some((dictionary, corrections)) = live_hints else {
            return stt::create_provider(
                &config.stt_provider,
                custom_whisper,
                Some(self.client.clone()),
            );
        };
        // Clients of the owner's recognition PC (Mac, home) run live passes there.
        let backend = match custom_whisper.as_ref() {
            _ if config.stt_provider == crate::engine_profile::OPENAI_CREDENTIAL => {
                ultra_local::LiveBackend::OpenAi {
                    token: stt_config.api_key.clone(),
                    client: self.client.clone(),
                }
            }
            Some(remote) if config.stt_custom_preset == stt::config::ULTRA_REMOTE_STT_PRESET => {
                ultra_local::LiveBackend::Remote {
                    endpoint: ultra_local::live_endpoint_for(&remote.endpoint),
                    token: stt_config.api_key.clone(),
                    client: self.client.clone(),
                }
            }
            _ => ultra_local::LiveBackend::Local,
        };
        Ok(Box::new(ultra_local::UltraLocalLiveProvider::new(
            &dictionary,
            corrections,
            backend,
            config.live_idle_stop_seconds as usize * 1000,
        )))
    }
}
