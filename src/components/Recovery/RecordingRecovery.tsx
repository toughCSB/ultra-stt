import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { FolderOpen, Mic, RefreshCw } from 'lucide-react'
import { useAppStore } from '../../stores/appStore'
import {
  copyRecordingTranscript,
  deleteRecordingArchive,
  listRecordingArchives,
  openRecordingArchiveFolder,
  retryRecordingArchive,
  type ArchivedRecording,
} from '../../lib/tauri'

export function RecordingRecovery() {
  const { t, i18n } = useTranslation()
  const [entries, setEntries] = useState<ArchivedRecording[]>([])
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState('')
  const [confirmDelete, setConfirmDelete] = useState('')
  const [revision, setRevision] = useState(0)
  const [confirmExternal, setConfirmExternal] = useState('')
  const externalEngine = useAppStore(
    (state) => (state.savedConfig ?? state.config).engine_mode === 'openai',
  )

  useEffect(() => {
    let cancelled = false
    const load = async () => {
      try {
        const items = await listRecordingArchives()
        if (!cancelled) setEntries(items)
      } catch {
        if (!cancelled) setError(t('audioRecovery.loadFailed'))
      }
    }
    void load()
    const timer = setInterval(() => void load(), 5000)
    return () => {
      cancelled = true
      clearInterval(timer)
    }
  }, [revision, t])

  const action = async (id: string, work: () => Promise<unknown>) => {
    setBusy(id)
    setError('')
    try {
      await work()
      setRevision((value) => value + 1)
    } catch (failure) {
      setError(`${t('audioRecovery.failed')} ${String(failure)}`)
    } finally {
      setBusy(null)
    }
  }

  return (
    <section
      className="mx-6 mt-6 rounded-xl border border-accent/30 bg-accent/5 p-4"
      aria-label={t('audioRecovery.title')}
    >
      <div className="flex items-center justify-between gap-3">
        <h3 className="flex items-center gap-2 text-sm font-semibold">
          <Mic size={16} />
          {t('audioRecovery.title')}
        </h3>
        <button
          type="button"
          onClick={() => void action('folder', openRecordingArchiveFolder)}
          disabled={busy !== null}
          className="flex items-center gap-1 text-xs text-accent disabled:opacity-50"
        >
          <FolderOpen size={14} />
          {t('audioRecovery.folder')}
        </button>
      </div>
      <p className="mt-2 text-xs leading-relaxed text-text-secondary">
        {t('audioRecovery.description')}
      </p>
      {externalEngine && (
        <p className="mt-2 text-xs leading-relaxed text-text-secondary">
          {t('audioRecovery.externalNotice')}
        </p>
      )}
      {entries.length === 0 && (
        <p className="mt-3 text-xs text-text-secondary">{t('audioRecovery.empty')}</p>
      )}
      <ol className="divide-y divide-border">
        {entries.map((entry) => (
          <li key={entry.id} className="pt-4 pb-2">
            <div className="flex flex-wrap items-center gap-2 text-xs text-text-secondary">
              <time>{new Date(entry.createdAt).toLocaleString(i18n.language)}</time>
              <span>
                {Math.floor(entry.durationSeconds / 60)}:
                {String(entry.durationSeconds % 60).padStart(2, '0')}
              </span>
              {entry.active && <span>{t('audioRecovery.active')}</span>}
            </div>
            {entry.transcript && (
              <p className="mt-2 max-h-64 overflow-y-auto whitespace-pre-wrap break-words text-sm select-text">
                {entry.transcript}
              </p>
            )}
            <div className="mt-3 flex flex-wrap items-center gap-3 text-xs">
              <button
                type="button"
                disabled={busy !== null || entry.active}
                onClick={() => {
                  if (externalEngine && confirmExternal !== entry.id) {
                    setConfirmExternal(entry.id)
                    return
                  }
                  void action(entry.id, () =>
                    externalEngine
                      ? retryRecordingArchive(entry.id, true)
                      : retryRecordingArchive(entry.id),
                  )
                  setConfirmExternal('')
                }}
                className="flex items-center gap-1 rounded-lg bg-accent px-3 py-2 text-white disabled:opacity-50"
              >
                <RefreshCw size={13} className={busy === entry.id ? 'animate-spin' : ''} />
                {busy === entry.id
                  ? t('audioRecovery.working')
                  : externalEngine
                    ? t(
                        confirmExternal === entry.id
                          ? 'audioRecovery.confirmExternal'
                          : 'audioRecovery.retryExternal',
                      )
                    : t('audioRecovery.retry')}
              </button>
              {entry.transcript && (
                <button
                  type="button"
                  disabled={busy !== null}
                  onClick={() =>
                    void action(entry.id, async () => {
                      await copyRecordingTranscript(entry.id)
                      setCopied(entry.id)
                    })
                  }
                >
                  {t(copied === entry.id ? 'recovery.copied' : 'recovery.copy')}
                </button>
              )}
              <button
                type="button"
                disabled={busy !== null || entry.active}
                onClick={() => {
                  if (confirmDelete !== entry.id) {
                    setConfirmDelete(entry.id)
                    return
                  }
                  void action(entry.id, () => deleteRecordingArchive(entry.id))
                  setConfirmDelete('')
                }}
                className="text-text-secondary disabled:opacity-50"
              >
                {t(
                  confirmDelete === entry.id
                    ? 'audioRecovery.confirmDelete'
                    : 'audioRecovery.delete',
                )}
              </button>
            </div>
          </li>
        ))}
      </ol>
      {error && (
        <p role="alert" className="mt-3 break-words text-xs text-error">
          {error}
        </p>
      )}
    </section>
  )
}
