import { useTranslation } from 'react-i18next'
import { abortRecording } from '../../lib/tauri'
import { useAppStore } from '../../stores/appStore'
import { CapsuleCancelButton, CapsuleStage } from './CapsuleStage'
import { CapsuleWorkIndicator } from './CapsuleWorkIndicator'

export function CapsuleProcessing() {
  const { t } = useTranslation()
  const partialTranscript = useAppStore((state) => state.partialTranscript)
  return (
    <CapsuleStage
      status={t('capsule.transcribing')}
      detail={
        partialTranscript ? (
          <span className="capsule-stage-transcript" title={partialTranscript}>
            {partialTranscript}
          </span>
        ) : (
          <CapsuleWorkIndicator />
        )
      }
      actions={
        <CapsuleCancelButton label={t('capsule.cancelProcessing')} onCancel={abortRecording} />
      }
    />
  )
}
