import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { CapsuleStage } from './CapsuleStage'

export function CapsuleError() {
  const { t } = useTranslation()
  const pipelineError = useAppStore((s) => s.pipelineError)
  const setPipelineError = useAppStore((s) => s.setPipelineError)
  const resetRecording = useAppStore((s) => s.resetRecording)

  useEffect(() => {
    const timer = setTimeout(() => {
      setPipelineError(null)
      // Only reset recording state if the pipeline is actually idle.
      // If the user started a new recording during the 2.5s error window,
      // don't overwrite the active pipeline state.
      const currentState = useAppStore.getState().pipelineState
      if (currentState === 'idle') {
        resetRecording()
      }
    }, 2500)
    return () => clearTimeout(timer)
  }, [setPipelineError, resetRecording, pipelineError])

  return (
    <CapsuleStage
      tone="error"
      status={t('capsule.errorOccurred')}
      detail={
        <span className="capsule-stage-transcript" title={pipelineError || undefined}>
          {pipelineError || t('capsule.errors.unknown')}
        </span>
      }
    />
  )
}
