import { useTranslation } from 'react-i18next'
import { abortRecording } from '../../lib/tauri'
import { CapsuleCancelButton, CapsuleStage } from './CapsuleStage'
import { CapsuleWorkIndicator } from './CapsuleWorkIndicator'

export function CapsulePolishing() {
  const { t } = useTranslation()
  return (
    <CapsuleStage
      status={t('capsule.thinking')}
      detail={<CapsuleWorkIndicator tone="thinking" />}
      actions={
        <CapsuleCancelButton label={t('capsule.cancelPolishing')} onCancel={abortRecording} />
      }
    />
  )
}
