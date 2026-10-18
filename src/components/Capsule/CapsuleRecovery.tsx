import { useState } from 'react'
import { AlertCircle, Check, ClipboardCopy, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { copyRecoveryEntry } from '../../lib/tauri'
import { useAppStore, type RecoveryEntry } from '../../stores/appStore'
import { CapsuleStage } from './CapsuleStage'

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
    <CapsuleStage
      status={t('recovery.capsuleNotice')}
      detail={
        <span className="capsule-stage-transcript" title={entry.text}>
          {entry.text}
        </span>
      }
      actions={
        <>
          <button
            type="button"
            disabled={busy}
            aria-label={t(
              failed ? 'recovery.copyFailed' : copied ? 'recovery.copied' : 'recovery.copy',
            )}
            title={t(failed ? 'recovery.copyFailed' : copied ? 'recovery.copied' : 'recovery.copy')}
            onPointerDown={(event) => event.stopPropagation()}
            onPointerUp={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation()
              void copy()
            }}
            className="capsule-action disabled:opacity-50"
          >
            {failed ? (
              <AlertCircle size={13} />
            ) : copied ? (
              <Check size={13} />
            ) : (
              <ClipboardCopy size={13} />
            )}
          </button>
          <button
            type="button"
            aria-label={t('recovery.hideNotice')}
            title={t('recovery.hideNotice')}
            onPointerDown={(event) => event.stopPropagation()}
            onPointerUp={(event) => event.stopPropagation()}
            onClick={(event) => {
              event.stopPropagation()
              clearAlert()
              setPipelineError(null)
            }}
            className="capsule-action"
          >
            <X size={13} />
          </button>
          {(copied || failed) && (
            <span role={failed ? 'alert' : 'status'} className="sr-only">
              {t(failed ? 'recovery.copyFailed' : 'recovery.copied')}
            </span>
          )}
        </>
      }
    />
  )
}
