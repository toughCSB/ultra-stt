import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Check, Loader2 } from 'lucide-react'
import { errorMessage } from '../../lib/errors'
import { getCredentialStatus, setCredential } from '../../lib/tauri'
import { FormField } from './shared/FormField'

type CredentialState = 'loading' | 'empty' | 'saved' | 'error'

export function OpenAiCredentialPanel() {
  const { t } = useTranslation()
  const [credentialState, setCredentialState] = useState<CredentialState>('loading')
  const [readError, setReadError] = useState<string | null>(null)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [retry, setRetry] = useState(0)
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    let active = true
    getCredentialStatus('stt', 'openai-transcribe')
      .then((status) => {
        if (!active) return
        if (status.storage === 'unavailable') {
          setReadError(null)
          setCredentialState('error')
        } else {
          setCredentialState(status.hasSecret ? 'saved' : 'empty')
        }
      })
      .catch((error) => {
        if (!active) return
        setReadError(errorMessage(error))
        setCredentialState('error')
      })
    return () => {
      active = false
    }
  }, [retry])

  const save = async () => {
    const value = draft.trim()
    if (!value || saving) return
    setSaving(true)
    setSaveError(null)
    try {
      await setCredential('stt', 'openai-transcribe', value)
      setCredentialState('saved')
      setEditing(false)
      setDraft('')
    } catch (error) {
      setSaveError(errorMessage(error))
    } finally {
      setSaving(false)
    }
  }

  const cancel = () => {
    setDraft('')
    setSaveError(null)
    setEditing(false)
  }

  return (
    <FormField label={t('settings.engine.apiKey')}>
      {credentialState === 'loading' && (
        <p role="status" className="flex items-center gap-2 text-[12px] text-text-secondary">
          <Loader2 size={14} className="animate-spin" aria-hidden="true" />
          {t('settings.engine.keyLoading')}
        </p>
      )}
      {credentialState === 'error' && (
        <div className="space-y-2">
          <p role="alert" className="text-[12px] text-error">
            {readError
              ? t('settings.engine.keyReadFailed', { details: readError })
              : t('settings.engine.keyVaultUnavailable')}
          </p>
          <button
            type="button"
            onClick={() => {
              setReadError(null)
              setCredentialState('loading')
              setRetry((attempt) => attempt + 1)
            }}
            className="rounded-[10px] border border-border px-3 py-2 text-[13px] text-text-primary hover:bg-bg-tertiary"
          >
            {t('settings.engine.retry')}
          </button>
        </div>
      )}
      {(credentialState === 'empty' || credentialState === 'saved') && (
        <div className="space-y-3">
          {credentialState === 'saved' && (
            <div
              className={`flex flex-wrap items-center gap-3 rounded-[10px] border p-3 ${saveError ? 'border-warning/30 bg-warning/10' : 'border-success/30 bg-success/10'}`}
            >
              <span
                className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[12px] font-semibold ${saveError ? 'bg-warning/15 text-warning' : 'bg-success/15 text-success'}`}
              >
                {!saveError && <Check size={14} aria-hidden="true" />}
                {saveError ? t('settings.engine.previousKeySaved') : t('settings.engine.keySaved')}
              </span>
              <span
                className="min-w-0 flex-1 text-[13px] tracking-widest text-text-secondary"
                aria-label={t('settings.engine.keyMasked')}
              >
                ••••••••
              </span>
              {!editing && (
                <button
                  type="button"
                  onClick={() => {
                    setDraft('')
                    setSaveError(null)
                    setEditing(true)
                  }}
                  className="rounded-[10px] border border-border bg-bg-secondary px-3 py-2 text-[13px] text-text-primary hover:bg-bg-tertiary"
                >
                  {t('settings.engine.modifyKey')}
                </button>
              )}
            </div>
          )}
          {credentialState === 'empty' && !editing && (
            <p role="status" className="text-[12px] text-text-secondary">
              {t('settings.engine.keyNotSaved')}
            </p>
          )}
          {(credentialState === 'empty' || editing) && (
            <div className="space-y-2">
              <input
                type="password"
                autoComplete="new-password"
                aria-label={t('settings.engine.apiKey')}
                value={draft}
                disabled={saving}
                onChange={(event) => setDraft(event.target.value)}
                placeholder={t('settings.enterApiKey')}
                className="w-full px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
              />
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  onClick={save}
                  disabled={!draft.trim() || saving}
                  className="rounded-[10px] bg-accent px-4 py-2.5 text-[13px] font-medium text-white hover:opacity-90 disabled:opacity-50"
                >
                  {saving ? t('common.saving') : t('settings.engine.saveKey')}
                </button>
                {editing && (
                  <button
                    type="button"
                    onClick={cancel}
                    disabled={saving}
                    className="rounded-[10px] border border-border px-4 py-2.5 text-[13px] text-text-primary hover:bg-bg-tertiary disabled:opacity-50"
                  >
                    {t('settings.engine.cancelKeyEdit')}
                  </button>
                )}
              </div>
            </div>
          )}
          {saveError && (
            <p role="alert" className="text-[12px] text-error">
              {t('settings.credentialSaveFailed', { details: saveError })}
            </p>
          )}
          {credentialState === 'saved' && !saveError && (
            <p className="text-[12px] text-text-secondary">
              {t('settings.engine.keyStoredLocallyUnverified')}
            </p>
          )}
        </div>
      )}
    </FormField>
  )
}
