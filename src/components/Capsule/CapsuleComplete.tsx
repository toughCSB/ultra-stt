import { useTranslation } from 'react-i18next'
import { Check } from 'lucide-react'
import { useEffect } from 'react'
import { useAppStore } from '../../stores/appStore'
import { CapsuleStage } from './CapsuleStage'

export function CapsuleComplete() {
  const { t } = useTranslation()
  const resetRecording = useAppStore((s) => s.resetRecording)
  const setPipelineState = useAppStore((s) => s.setPipelineState)

  useEffect(() => {
    const timer = setTimeout(() => {
      resetRecording()
      setPipelineState('idle')
    }, 1200)
    return () => clearTimeout(timer)
  }, [resetRecording, setPipelineState])

  return (
    <CapsuleStage
      tone="complete"
      status={t('capsule.done')}
      detail={<Check size={20} strokeWidth={2.2} />}
    />
  )
}
