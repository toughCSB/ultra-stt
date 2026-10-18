import { useTranslation } from 'react-i18next'
import { abortRecording } from '../../lib/tauri'
import { CapsuleCancelButton, CapsuleStage } from './CapsuleStage'
import { CapsuleWorkIndicator } from './CapsuleWorkIndicator'

export function CapsulePreparing() {
  const { t } = useTranslation()
  return (
    <CapsuleStage
      status={t('capsule.preparing')}
      detail={<CapsuleWorkIndicator />}
      actions={
        <CapsuleCancelButton label={t('capsule.cancelRecording')} onCancel={abortRecording} />
      }
    />
  )
}
