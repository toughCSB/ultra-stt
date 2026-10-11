import { useRef, useState } from 'react'
import { ClipboardCopy, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { copyRecoveryEntry, dismissRecoveryEntry } from '../../lib/tauri'
import { useAppStore, type RecoveryEntry } from '../../stores/appStore'
import { RecordingRecovery } from './RecordingRecovery'

export function Recovery() {
  const { t, i18n } = useTranslation()
  const entries = useAppStore((state) => state.recoveryEntries)
  const removeEntry = useAppStore((state) => state.removeRecoveryEntry)
  const [busyId, setBusyId] = useState<string | null>(null)
  const [copiedId, setCopiedId] = useState<string | null>(null)
  const [error, setError] = useState<{ id: string; key: string } | null>(null)
  const copySequence = useRef(0)

  const copy = async (entry: RecoveryEntry) => {
    const sequence = ++copySequence.current
    setBusyId(entry.id)
    setError(null)
    try {
      await copyRecoveryEntry(entry.id)
      if (
        sequence === copySequence.current &&
        useAppStore.getState().recoveryEntries.some((item) => item.id === entry.id)
      ) {
        setCopiedId(entry.id)
      }
    } catch {
      if (sequence === copySequence.current) setError({ id: entry.id, key: 'recovery.copyFailed' })
    } finally {
      setBusyId((current) => (current === entry.id ? null : current))
    }
  }

  const dismiss = async (entry: RecoveryEntry) => {
    ++copySequence.current
    setBusyId(entry.id)
    setError(null)
    setCopiedId(null)
    try {
      await dismissRecoveryEntry(entry.id)
      removeEntry(entry.id)
    } catch {
      setError({ id: entry.id, key: 'recovery.dismissFailed' })
    } finally {
      setBusyId((current) => (current === entry.id ? null : current))
    }
  }

  return (
    <div className="w-full min-h-full bg-bg-primary text-text-primary">
      <header className="px-6 pt-6 pb-5 border-b border-border">
        <div className="flex items-baseline justify-between gap-4">
          <h2 className="text-[17px] font-semibold">{t('recovery.title')}</h2>
          <span className="text-[12px] tabular-nums text-text-secondary">
            {t('recovery.pendingCount', { count: entries.length })}
          </span>
        </div>
        <p className="mt-2 text-[12px] leading-relaxed text-text-secondary">
          {t('recovery.description')}
        </p>
      </header>

      <RecordingRecovery />

      {entries.length === 0 ? (
        <div className="px-6 py-14">
          <p className="text-[14px] font-medium">{t('recovery.empty')}</p>
          <p className="mt-1 text-[12px] text-text-secondary">{t('recovery.emptyHint')}</p>
        </div>
      ) : (
        <ol className="px-6">
          {entries.map((entry) => {
            const date = new Date(entry.createdAt)
            const time = Number.isNaN(date.getTime())
              ? entry.createdAt
              : date.toLocaleString(i18n.language)
            const actionBusy = busyId === entry.id
            return (
              <li key={entry.id} className="border-b border-border py-5 last:border-b-0">
                <div className="flex items-center gap-2 text-[11px] text-text-secondary">
                  <time dateTime={entry.createdAt}>{time}</time>
                  {entry.appName && (
                    <>
                      <span aria-hidden="true">·</span>
                      <span>{entry.appName}</span>
                    </>
                  )}
                  <span aria-hidden="true">·</span>
                  <span>{t(`recovery.reasons.${entry.reason}`)}</span>
                </div>
                <p className="mt-3 whitespace-pre-wrap break-words text-[14px] leading-[1.65] select-text">
                  {entry.text}
                </p>
                {(entry.reason === 'partial_insert' || entry.charsInserted > 0) && (
                  <p className="mt-2 text-[12px] text-text-secondary">
                    {t('recovery.partialHint')}
                  </p>
                )}
                <div className="mt-4 flex items-center gap-3">
                  <button
                    type="button"
                    disabled={actionBusy}
                    onClick={() => void copy(entry)}
                    className="inline-flex items-center gap-1.5 rounded-[8px] bg-accent px-3 py-1.5 text-[12px] font-medium text-white hover:bg-accent-hover disabled:opacity-50"
                  >
                    <ClipboardCopy size={14} />
                    {copiedId === entry.id ? t('recovery.copied') : t('recovery.copy')}
                  </button>
                  <button
                    type="button"
                    disabled={actionBusy}
                    onClick={() => void dismiss(entry)}
                    className="inline-flex items-center gap-1.5 rounded-[8px] px-2 py-1.5 text-[12px] text-text-secondary hover:bg-bg-secondary hover:text-text-primary disabled:opacity-50"
                  >
                    <X size={14} />
                    {t('recovery.dismiss')}
                  </button>
                </div>
                {error?.id === entry.id && (
                  <p role="alert" className="mt-2 text-[12px] text-error">
                    {t(error.key)}
                  </p>
                )}
              </li>
            )
          })}
        </ol>
      )}
    </div>
  )
}
