import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useTranslation } from 'react-i18next'
import { errorMessage } from '../../lib/errors'
import type { DesktopPolishModel } from '../../lib/desktop-polish-models'
import { testEngineSetup } from '../../lib/tauri'

interface Props {
  kind: 'stt' | 'llm'
  engineMode: 'existing' | 'openai'
  desktopPolishModel: DesktopPolishModel
  disabled?: boolean
}

type TestState = 'idle' | 'testing' | 'success' | 'error'

export function EngineConnectionTest({
  kind,
  engineMode,
  desktopPolishModel,
  disabled = false,
}: Props) {
  const { t } = useTranslation()
  const [state, setState] = useState<TestState>('idle')
  const [message, setMessage] = useState('')
  const requestId = useRef(0)

  useEffect(() => {
    let active = true
    let unlisten: (() => void) | undefined
    listen('credentials:changed', () => {
      requestId.current += 1
      setState('idle')
      setMessage('')
    })
      .then((stop) => {
        if (active) unlisten = stop
        else stop()
      })
      .catch(() => {})
    return () => {
      active = false
      requestId.current += 1
      unlisten?.()
    }
  }, [])

  const testConnection = async () => {
    if (disabled || state === 'testing') return
    const currentRequest = ++requestId.current
    setState('testing')
    setMessage('')
    try {
      const result = await testEngineSetup(kind, engineMode, desktopPolishModel)
      if (currentRequest !== requestId.current) return
      setState(result.success ? 'success' : 'error')
      setMessage(result.message)
    } catch (error) {
      if (currentRequest !== requestId.current) return
      setState('error')
      setMessage(errorMessage(error))
    }
  }

  return (
    <div className="space-y-2">
      <button
        type="button"
        onClick={testConnection}
        disabled={disabled || state === 'testing'}
        aria-busy={state === 'testing'}
        className="rounded-[10px] border border-border bg-bg-secondary px-4 py-2.5 text-[13px] font-medium text-text-primary hover:bg-bg-tertiary disabled:opacity-50"
      >
        {state === 'testing'
          ? t('settings.engine.testingConnection')
          : t(
              kind === 'stt'
                ? 'settings.engine.testSttConnection'
                : 'settings.engine.testLlmConnection',
            )}
      </button>
      {state === 'success' && (
        <p role="status" className="text-[12px] text-success">
          {t('settings.engine.testPassed')}
          {message && `: ${message}`}
        </p>
      )}
      {state === 'error' && (
        <p role="alert" className="text-[12px] text-error">
          {t('settings.engine.testFailed')}
          {message && `: ${message}`}
        </p>
      )}
      <p className="text-[11px] leading-relaxed text-text-tertiary">
        {t('settings.engine.connectionTestNote')}
      </p>
    </div>
  )
}
