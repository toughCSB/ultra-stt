import { useTranslation } from 'react-i18next'
import { abortAskDictation } from '../../lib/tauri'
import { CapsuleCancelButton, CapsuleStage } from './CapsuleStage'
import { DurationTimer } from './DurationTimer'
import { Waveform } from './Waveform'

export function CapsuleAskRecording() {
  const { t } = useTranslation()
  return (
    <CapsuleStage
      tone="recording"
      status={<span className="whitespace-nowrap">{t('ask.title')}</span>}
      timer={<DurationTimer recordingKind="ask" />}
      detail={<Waveform />}
      actions={
        <CapsuleCancelButton label={t('capsule.cancelRecording')} onCancel={abortAskDictation} />
      }
    />
  )
}
