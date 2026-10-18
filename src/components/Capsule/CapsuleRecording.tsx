import { useTranslation } from 'react-i18next'
import { AudioLines, Zap, X } from 'lucide-react'
import { abortRecording, switchRecordingMode } from '../../lib/tauri'
import { useAppStore } from '../../stores/appStore'
import { CapsuleStage } from './CapsuleStage'
import { DurationTimer } from './DurationTimer'
import { TranslateTargetChip } from './TranslateTargetChip'
import { Waveform } from './Waveform'

export function CapsuleRecording() {
  const { t } = useTranslation()
  const liveTyping = useAppStore((state) => state.liveTyping)
  const polishEnabled = useAppStore((state) => state.config.polish_enabled)

  const handleCancel = async (event: React.MouseEvent) => {
    event.stopPropagation()
    try {
      await abortRecording()
    } catch (error) {
      console.error('Failed to abort recording:', error)
    }
  }

  const handleSwitchMode = async (event: React.MouseEvent) => {
    event.stopPropagation()
    try {
      await switchRecordingMode(!liveTyping)
    } catch (error) {
      console.error('Failed to switch recording mode:', error)
    }
  }

  const stopPointerPropagation = (event: React.PointerEvent) => event.stopPropagation()

  return (
    <CapsuleStage
      tone={liveTyping ? 'live' : 'recording'}
      status={
        <>
          <span className="capsule-status-light" />
          <span className="capsule-status-label">
            {liveTyping ? t('capsule.liveTyping') : t('ask.listening')}
          </span>
          <span
            className="capsule-state-badge"
            title={!liveTyping && polishEnabled ? `STT · ${t('settings.aiPolish')}` : undefined}
          >
            {liveTyping ? (
              <>
                <Zap size={12} />
                LIVE
              </>
            ) : polishEnabled ? (
              'STT · AI'
            ) : (
              'STT'
            )}
          </span>
          {!liveTyping && <TranslateTargetChip />}
        </>
      }
      timer={<DurationTimer />}
      detail={<Waveform />}
      actions={
        <>
          <button
            type="button"
            onPointerDown={stopPointerPropagation}
            onPointerUp={stopPointerPropagation}
            onClick={handleSwitchMode}
            title={t(liveTyping ? 'capsule.switchToStt' : 'capsule.switchToLive')}
            aria-label={t(liveTyping ? 'capsule.switchToStt' : 'capsule.switchToLive')}
            className="capsule-action capsule-action-mode"
          >
            {liveTyping ? (
              <AudioLines size={22} strokeWidth={2.2} />
            ) : (
              <Zap size={22} strokeWidth={2.2} />
            )}
            <span>{liveTyping ? 'STT' : 'LIVE'}</span>
          </button>
          <button
            type="button"
            onPointerDown={stopPointerPropagation}
            onPointerUp={stopPointerPropagation}
            onClick={handleCancel}
            aria-label={t('capsule.cancelRecording')}
            className="capsule-action capsule-action-cancel"
          >
            <X size={20} strokeWidth={2.2} />
          </button>
        </>
      }
    />
  )
}
