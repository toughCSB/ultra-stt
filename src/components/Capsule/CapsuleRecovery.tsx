import { useState } from 'react'
import { AlertCircle, Check, ClipboardCopy, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { copyRecoveryEntry } from '../../lib/tauri'
import { useAppStore, type RecoveryEntry } from '../../stores/appStore'

export function CapsuleRecovery({ entry }: { entry: RecoveryEntry }) {
  const { t } = useTranslation()
  const clearAlert = useAppStore((state) => state.clearRecoveryAlert)
  const setPipelineError = useAppStore((state) => state.setPipelineError)
  const [busy, setBusy] = useState(false)
  const [copied, setCopied] = useState(false)
  const [failed, setFailed] = useState(false)

  const copy = async () => {
    setBusy(true)
    setFailed(false)
    try {
      await copyRecoveryEntry(entry.id)
      if (useAppStore.getState().recoveryAlertId === entry.id) setCopied(true)
    } catch {
      if (useAppStore.getState().recoveryAlertId === entry.id) setFailed(true)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="relative z-10 flex h-9 w-full items-center gap-2 pl-3 pr-1.5 text-white">
      <span className="min-w-0 flex-1 truncate text-[11px]" title={entry.text}>
        {t('recovery.capsuleNotice')} · {entry.text}
      </span>
      <button type="button" disabled={busy} aria-label={t(failed ? 'recovery.copyFailed' : copied ? 'recovery.copied' : 'recovery.copy')} title={t(failed ? 'recovery.copyFailed' : copied ? 'recovery.copied' : 'recovery.copy')} onPointerDown={(event) => event.stopPropagation()} onPointerUp={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); void copy() }} className="shrink-0 rounded-full bg-white/15 p-1.5 text-white hover:bg-white/25 disabled:opacity-50">
        {failed ? <AlertCircle size={13} /> : copied ? <Check size={13} /> : <ClipboardCopy size={13} />}
      </button>
      <button type="button" aria-label={t('recovery.hideNotice')} title={t('recovery.hideNotice')} onPointerDown={(event) => event.stopPropagation()} onPointerUp={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); clearAlert(); setPipelineError(null) }} className="shrink-0 rounded-full p-1.5 text-white/80 hover:bg-white/15">
        <X size={13} />
      </button>
      {(copied || failed) && <span role={failed ? 'alert' : 'status'} className="sr-only">{t(failed ? 'recovery.copyFailed' : 'recovery.copied')}</span>}
    </div>
  )
}
