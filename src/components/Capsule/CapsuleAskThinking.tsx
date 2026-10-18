import { useTranslation } from 'react-i18next'
import { abortAskDictation } from '../../lib/tauri'
import { CapsuleCancelButton, CapsuleStage } from './CapsuleStage'
import { CapsuleWorkIndicator } from './CapsuleWorkIndicator'

export function CapsuleAskThinking() {
  const { t } = useTranslation()
  return (
    <CapsuleStage
      status={<span className="whitespace-nowrap">{t('ask.title')}</span>}
      detail={
        <span className="capsule-stage-thought">
          {t('ask.thinking')} <CapsuleWorkIndicator tone="thinking" />
        </span>
      }
      actions={
        <CapsuleCancelButton label={t('capsule.cancelRecording')} onCancel={abortAskDictation} />
      }
    />
  )
}
