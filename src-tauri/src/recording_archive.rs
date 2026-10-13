use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::Manager;

const MAX_PCM: usize = 24 * 1024 * 1024;
const MAX_STORAGE: u64 = 512 * 1024 * 1024;
const MAX_ENTRIES: usize = 20;

#[derive(Clone)]
pub struct RecordingArchive {
    root: PathBuf,
    active: Arc<Mutex<HashSet<String>>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedRecording {
    pub id: String,
    pub created_at: String,
    pub duration_seconds: u64,
    pub transcript: String,
    pub active: bool,
}

struct Lease {
    id: String,
    active: Arc<Mutex<HashSet<String>>>,
}

impl Drop for Lease {
    fn drop(&mut self) {
        self.active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&self.id);
    }
}

pub struct RecordingJournal {
    archive: RecordingArchive,
    lease: Lease,
    file: Option<File>,
    bytes: usize,
    checkpoint_bytes: usize,
    discarded: bool,
}

fn failure(message: &str) -> io::Error {
    io::Error::other(message)
}

fn private_file(path: &Path, create_new: bool) -> io::Result<File> {
    if !create_new
        && fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(failure("Recovery file cannot be a symbolic link"));
    }
    let mut options = OpenOptions::new();
    options.write(true);
    if create_new {
        options.create_new(true);
    } else {
        options.create(true).truncate(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

impl RecordingArchive {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn prepare(&self) -> io::Result<()> {
        fs::create_dir_all(&self.root)?;
        if fs::symlink_metadata(&self.root)?.file_type().is_symlink() {
            return Err(failure("Recovery directory cannot be a symbolic link"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))?;
        }
        Ok(())
    }

    fn path(&self, id: &str, extension: &str) -> io::Result<PathBuf> {
        let parsed =
            uuid::Uuid::parse_str(id).map_err(|_| failure("Invalid recording identifier"))?;
        if parsed.to_string() != id {
            return Err(failure("Invalid recording identifier"));
        }
        Ok(self.root.join(format!("{id}.{extension}")))
    }

    fn claim(&self, id: &str) -> io::Result<Lease> {
        self.path(id, "wav")?;
        if !self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(id.to_string())
        {
            return Err(failure("This recording is still in use"));
        }
        Ok(Lease {
            id: id.to_string(),
            active: self.active.clone(),
        })
    }

    pub fn begin(&self) -> io::Result<RecordingJournal> {
        self.prepare()?;
        // A full store never blocks recording: the oldest saved recordings make room.
        loop {
            let entries = self.entries()?;
            let bytes = fs::read_dir(&self.root)?.try_fold(0u64, |total, entry| {
                Ok::<_, io::Error>(total.saturating_add(entry?.metadata()?.len()))
            })?;
            if entries.len() < MAX_ENTRIES && bytes.saturating_add(MAX_PCM as u64) <= MAX_STORAGE {
                break;
            }
            // `entries` is newest first; recordings still in use are never removed.
            let Some(oldest) = entries.iter().rev().find(|entry| !entry.active) else {
                return Err(failure(
                    "Recovery storage is full of recordings still in use.",
                ));
            };
            tracing::info!("Recovery storage full; removing the oldest saved recording");
            self.delete(&oldest.id)?;
        }
        let id = uuid::Uuid::new_v4().to_string();
        let lease = self.claim(&id)?;
        let mut file = private_file(&self.path(&id, "wav")?, true)?;
        file.write_all(&crate::stt::whisper_compat::WhisperCompatProvider::build_wav(&[], 16000))?;
        Ok(RecordingJournal {
            archive: self.clone(),
            lease,
            file: Some(file),
            bytes: 0,
            checkpoint_bytes: 0,
            discarded: false,
        })
    }

    pub fn entries(&self) -> io::Result<Vec<ArchivedRecording>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let active = self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut result = Vec::new();
        for item in fs::read_dir(&self.root)? {
            let item = item?;
            let path = item.path();
            if path.extension().and_then(|value| value.to_str()) != Some("wav")
                || !item.file_type()?.is_file()
            {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            if self.path(id, "wav").is_err() {
                continue;
            }
            let metadata = item.metadata()?;
            let text_path = self.path(id, "txt")?;
            let transcript = match fs::symlink_metadata(&text_path) {
                Ok(metadata) if metadata.is_file() && metadata.len() <= 1024 * 1024 => {
                    fs::read_to_string(text_path)?
                }
                _ => String::new(),
            };
            let created: chrono::DateTime<chrono::Utc> =
                metadata.created().or_else(|_| metadata.modified())?.into();
            result.push(ArchivedRecording {
                id: id.to_string(),
                created_at: created.to_rfc3339(),
                duration_seconds: metadata.len().saturating_sub(44) / 32000,
                transcript,
                active: active.contains(id),
            });
        }
        result.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(result)
    }

    fn read_pcm(&self, id: &str) -> io::Result<Vec<u8>> {
        let path = self.path(id, "wav")?;
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file() || !(46..=MAX_PCM as u64 + 44).contains(&metadata.len()) {
            return Err(failure("Invalid recovery audio size"));
        }
        let wav = fs::read(path)?;
        let expected = crate::stt::whisper_compat::WhisperCompatProvider::build_wav(&[], 16000);
        if wav[..4] != expected[..4] || wav[8..40] != expected[8..40] {
            return Err(failure("Invalid recovery audio format"));
        }
        let complete_end = wav.len() - (wav.len() - 44) % 2;
        Ok(wav[44..complete_end].to_vec())
    }

    fn save_transcript(&self, id: &str, text: &str) -> io::Result<()> {
        if text.len() > 1024 * 1024 {
            return Err(failure("Recovery transcript is too large"));
        }
        let mut file = private_file(&self.path(id, "txt")?, false)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()
    }

    fn delete_files(&self, id: &str) -> io::Result<()> {
        for extension in ["wav", "txt"] {
            match fs::remove_file(self.path(id, extension)?) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> io::Result<()> {
        let _lease = self.claim(id)?;
        self.delete_files(id)
    }
}

impl RecordingJournal {
    pub fn append(&mut self, pcm: &[u8]) -> io::Result<()> {
        if self.discarded {
            return Err(failure("Recording was cancelled"));
        }
        if self.bytes.saturating_add(pcm.len()) > MAX_PCM || !pcm.len().is_multiple_of(2) {
            return Err(failure("Recovery PCM limit exceeded"));
        }
        self.file
            .as_mut()
            .ok_or_else(|| failure("Recording already stopped"))?
            .write_all(pcm)?;
        self.bytes += pcm.len();
        if self.bytes - self.checkpoint_bytes >= 32_000 {
            self.checkpoint()?;
        }
        Ok(())
    }

    fn checkpoint(&mut self) -> io::Result<()> {
        if let Some(file) = self.file.as_mut() {
            file.seek(SeekFrom::Start(4))?;
            file.write_all(&(self.bytes as u32 + 36).to_le_bytes())?;
            file.seek(SeekFrom::Start(40))?;
            file.write_all(&(self.bytes as u32).to_le_bytes())?;
            file.seek(SeekFrom::End(0))?;
            file.sync_data()?;
            self.checkpoint_bytes = self.bytes;
        }
        Ok(())
    }

    pub fn finish_audio(&mut self) -> io::Result<()> {
        self.checkpoint()?;
        self.file = None;
        Ok(())
    }

    pub fn save_transcript(&self, text: &str) -> io::Result<()> {
        if self.discarded {
            return Ok(());
        }
        self.archive.save_transcript(&self.lease.id, text)
    }

    pub fn discard(&mut self) -> io::Result<()> {
        self.file = None;
        self.discarded = true;
        self.archive.delete_files(&self.lease.id)
    }
}

impl Drop for RecordingJournal {
    fn drop(&mut self) {
        if let Err(error) = self.finish_audio() {
            tracing::error!("Recovery audio checkpoint failed: {error}");
        }
    }
}

#[tauri::command]
pub fn list_recording_archives(
    store: tauri::State<'_, RecordingArchive>,
) -> Result<Vec<ArchivedRecording>, String> {
    store.entries().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_recording_archive(
    store: tauri::State<'_, RecordingArchive>,
    id: String,
) -> Result<(), String> {
    store.delete(&id).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn copy_recording_transcript(app: tauri::AppHandle, id: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let store = app.state::<RecordingArchive>();
    let entry = store
        .entries()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|entry| entry.id == id && !entry.transcript.is_empty())
        .ok_or("No saved transcript")?;
    app.clipboard()
        .write_text(entry.transcript)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_recording_archive_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let store = app.state::<RecordingArchive>();
    store.prepare().map_err(|error| error.to_string())?;
    app.opener()
        .open_path(store.root.to_string_lossy(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn retry_recording_archive(app: tauri::AppHandle, id: String) -> Result<String, String> {
    let store = app.state::<RecordingArchive>().inner().clone();
    let config = app
        .state::<crate::storage::ConfigManager>()
        .load()
        .await
        .map_err(|error| error.to_string())?;
    if config.stt_provider != crate::stt::config::CUSTOM_WHISPER_PROVIDER {
        return Err("Retry uses the configured Local / Custom Whisper provider. The original WAV remains available in the recovery folder.".into());
    }
    let whisper = crate::stt::config::build_custom_whisper_for_app(&config)?;
    let mut provider = crate::stt::create_provider(
        &config.stt_provider,
        Some(whisper),
        Some(app.state::<reqwest::Client>().inner().clone()),
    )
    .map_err(|error| error.to_string())?;
    let secret = crate::credentials::resolve_stt_config_secret(
        &config,
        &crate::credentials::SystemCredentialVault,
    )
    .map_err(|error| error.to_string())?;
    let stt_config = crate::stt::SttConfig {
        api_key: secret,
        language: (config.stt_language != "multi").then_some(config.stt_language),
        ..Default::default()
    };
    let options = (config.stt_custom_preset == crate::stt::config::ULTRA_REMOTE_STT_PRESET)
        .then_some(crate::stt::RemotePolishOptions {
            style: config.polish_style,
            enabled: false,
        });
    transcribe_saved_audio(&store, &id, provider.as_mut(), &stt_config, options).await
}

async fn transcribe_saved_audio(
    store: &RecordingArchive,
    id: &str,
    provider: &mut dyn crate::stt::SttProvider,
    config: &crate::stt::SttConfig,
    options: Option<crate::stt::RemotePolishOptions>,
) -> Result<String, String> {
    let _lease = store.claim(id).map_err(|error| error.to_string())?;
    let pcm = store.read_pcm(id).map_err(|error| error.to_string())?;
    let work = async {
        provider.connect(config).await?;
        if let Some(options) = options {
            provider.set_remote_polish(&options);
        }
        provider.send_audio(&pcm).await?;
        provider.disconnect().await
    };
    let budget = crate::stt::processing_budget::finalize(
        crate::stt::processing_budget::pcm_seconds(pcm.len(), 16000),
    );
    let text = tokio::time::timeout(budget, work)
        .await
        .map_err(|_| "Transcription timed out; original audio is preserved".to_string())?
        .map_err(|error| error.to_string())?
        .filter(|text| !text.trim().is_empty())
        .ok_or("No speech recognized; original audio is preserved")?;
    store
        .save_transcript(id, &text)
        .map_err(|error| error.to_string())?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct RetryProvider {
        fail: bool,
        received: Vec<u8>,
        options: Option<crate::stt::RemotePolishOptions>,
    }

    #[async_trait::async_trait]
    impl crate::stt::SttProvider for RetryProvider {
        async fn connect(
            &mut self,
            _: &crate::stt::SttConfig,
        ) -> Result<(), crate::error::AppError> {
            Ok(())
        }
        async fn send_audio(&mut self, pcm: &[u8]) -> Result<(), crate::error::AppError> {
            self.received.extend_from_slice(pcm);
            Ok(())
        }
        async fn recv_transcript(
            &mut self,
        ) -> Result<Option<crate::stt::TranscriptEvent>, crate::error::AppError> {
            Ok(None)
        }
        async fn disconnect(&mut self) -> Result<Option<String>, crate::error::AppError> {
            if self.fail {
                Err(crate::error::AppError::Config("server unavailable".into()))
            } else {
                Ok(Some("재전사한 원문".into()))
            }
        }
        fn set_remote_polish(&mut self, options: &crate::stt::RemotePolishOptions) {
            self.options = Some(options.clone());
        }
        fn name(&self) -> &str {
            "recovery-test"
        }
    }

    #[tokio::test]
    async fn failed_retry_preserves_audio_and_text_then_success_keeps_audio_until_explicit_delete()
    {
        let store = archive();
        let mut journal = store.begin().unwrap();
        let id = journal.lease.id.clone();
        journal.append(&[1, 2, 3, 4]).unwrap();
        journal.save_transcript("기존 원문").unwrap();
        drop(journal);
        let mut provider = RetryProvider {
            fail: true,
            received: vec![],
            options: None,
        };
        let config = crate::stt::SttConfig::default();
        assert!(
            transcribe_saved_audio(&store, &id, &mut provider, &config, None)
                .await
                .is_err()
        );
        assert_eq!(store.read_pcm(&id).unwrap(), vec![1, 2, 3, 4]);
        let entries = store.entries().unwrap();
        assert_eq!(entries[0].transcript, "기존 원문");
        assert!(!entries[0].active);
        provider.fail = false;
        provider.received.clear();
        let options = crate::stt::RemotePolishOptions {
            style: "minimal".into(),
            enabled: false,
        };
        assert_eq!(
            transcribe_saved_audio(&store, &id, &mut provider, &config, Some(options))
                .await
                .unwrap(),
            "재전사한 원문"
        );
        assert_eq!(provider.received, vec![1, 2, 3, 4]);
        assert!(!provider.options.unwrap().enabled);
        assert_eq!(store.entries().unwrap()[0].transcript, "재전사한 원문");
        assert_eq!(store.read_pcm(&id).unwrap(), vec![1, 2, 3, 4]);
        store.delete(&id).unwrap();
        fs::remove_dir(&store.root).unwrap();
    }

    fn archive() -> RecordingArchive {
        RecordingArchive::new(
            std::env::temp_dir().join(format!("ultra-stt-recovery-test-{}", uuid::Uuid::new_v4())),
        )
    }

    #[test]
    fn ten_minute_audio_survives_failure_and_app_restart_with_exact_bytes() {
        let store = archive();
        let mut journal = store.begin().unwrap();
        let id = journal.lease.id.clone();
        let chunk: Vec<u8> = (0..32000).map(|index| (index % 253) as u8).collect();
        for _ in 0..600 {
            journal.append(&chunk).unwrap();
        }
        assert!(store.delete(&id).is_err());
        journal.finish_audio().unwrap();
        journal
            .save_transcript("전사 후 다듬기 실패에도 이 원문을 보존")
            .unwrap();
        drop(journal);
        let restarted = RecordingArchive::new(store.root.clone());
        let audio = restarted.read_pcm(&id).unwrap();
        assert_eq!(audio, chunk.repeat(600));
        let entries = restarted.entries().unwrap();
        assert_eq!(entries[0].duration_seconds, 600);
        assert!(!entries[0].active);
        assert_eq!(
            entries[0].transcript,
            "전사 후 다듬기 실패에도 이 원문을 보존"
        );
        restarted.delete(&id).unwrap();
        fs::remove_dir(&store.root).unwrap();
    }

    #[test]
    fn success_or_explicit_cancel_removes_only_the_owned_recording() {
        let store = archive();
        let mut first = store.begin().unwrap();
        let mut second = store.begin().unwrap();
        first.append(&[1, 0]).unwrap();
        second.append(&[2, 0]).unwrap();
        first.discard().unwrap();
        assert!(first.append(&[3, 0]).is_err());
        first.save_transcript("late result").unwrap();
        assert_eq!(store.entries().unwrap().len(), 1);
        second.discard().unwrap();
        drop(first);
        drop(second);
        assert!(store.entries().unwrap().is_empty());
        fs::remove_dir(&store.root).unwrap();
    }

    #[test]
    fn recovery_rejects_path_traversal_and_double_retry() {
        let store = archive();
        assert!(store.path("../settings", "wav").is_err());
        let mut journal = store.begin().unwrap();
        assert!(store.claim(&journal.lease.id).is_err());
        journal.discard().unwrap();
        drop(journal);
        fs::remove_dir(&store.root).unwrap();
    }

    #[test]
    fn full_store_removes_the_oldest_saved_recording_and_keeps_recording() {
        let store = archive();
        for _ in 0..MAX_ENTRIES {
            let mut journal = store.begin().unwrap();
            journal.append(&[1, 0]).unwrap();
        }
        let before = store.entries().unwrap();
        let oldest = before.last().unwrap().id.clone();

        let mut journal = store.begin().unwrap();
        journal.append(&[2, 0]).unwrap();
        let newest = journal.lease.id.clone();
        drop(journal);

        let entries = store.entries().unwrap();
        assert_eq!(entries.len(), MAX_ENTRIES);
        assert!(entries.iter().all(|entry| entry.id != oldest));
        assert_eq!(store.read_pcm(&newest).unwrap(), vec![2, 0]);
        for entry in entries {
            store.delete(&entry.id).unwrap();
        }
        fs::remove_dir(&store.root).unwrap();
    }

    #[test]
    fn incomplete_header_after_interruption_uses_actual_audio_bytes() {
        let store = archive();
        let mut journal = store.begin().unwrap();
        journal.append(&[1, 0, 2, 0]).unwrap();
        assert_eq!(store.read_pcm(&journal.lease.id).unwrap(), vec![1, 0, 2, 0]);
        journal.discard().unwrap();
        drop(journal);
        fs::remove_dir(&store.root).unwrap();
    }

    #[test]
    fn interrupted_final_sample_preserves_all_complete_pcm_samples() {
        let store = archive();
        let mut journal = store.begin().unwrap();
        let id = journal.lease.id.clone();
        journal.append(&[1, 0, 2, 0]).unwrap();
        drop(journal);
        OpenOptions::new()
            .append(true)
            .open(store.path(&id, "wav").unwrap())
            .unwrap()
            .write_all(&[3])
            .unwrap();
        assert_eq!(store.read_pcm(&id).unwrap(), vec![1, 0, 2, 0]);
        store.delete(&id).unwrap();
        fs::remove_dir(&store.root).unwrap();
    }
}
