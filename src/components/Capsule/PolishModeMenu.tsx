import { ChevronDown, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { polishModeLabelKey } from '../../lib/polishStyles'
import { PolishModeSelector } from '../PolishModeSelector'
import { togglePolishModeMenu } from '../../lib/capsulePolishMenu'
import { CapsuleBrand } from './CapsuleWorkIndicator'
import { DurationTimer } from './DurationTimer'

export function PolishModeChip() {
  const { t } = useTranslation()
  const style = useAppStore((state) => state.config.polish_style)
  const open = useAppStore((state) => state.polishModeMenuOpen)
  return (
    <button
      type="button"
      aria-label={t('settings.polishModeTitle')}
      title={`${t('settings.polishModeTitle')}: ${t(polishModeLabelKey(style))}`}
      aria-haspopup="dialog"
      aria-expanded={open}
      onPointerDown={(event) => event.stopPropagation()}
      onPointerUp={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation()
        togglePolishModeMenu()
      }}
      className="capsule-mode-chip flex items-center justify-center gap-1.5 rounded-full px-3 text-[12px] font-semibold"
    >
      <span className="truncate">{t(polishModeLabelKey(style))}</span>
      <ChevronDown size={17} className="shrink-0" />
    </button>
  )
}

export function PolishModeMenu() {
  const { t } = useTranslation()
  const open = useAppStore((state) => state.polishModeMenuOpen)
  const close = useAppStore((state) => state.setPolishModeMenuOpen)
  const pipelineState = useAppStore((state) => state.pipelineState)
  if (!open) return null
  return (
    <div
      role="dialog"
      aria-label={t('settings.polishModeTitle')}
      className="capsule-mode-menu capsule-mode-picker"
      onPointerDown={(event) => event.stopPropagation()}
      onPointerUp={(event) => event.stopPropagation()}
    >
      <CapsuleBrand size={54} />
      <div className="capsule-mode-picker-main">
        <div className="capsule-mode-picker-heading">
          <span>{t('settings.polishModeTitle')}</span>
          {pipelineState === 'recording' && <DurationTimer />}
          {['transcribing', 'polishing', 'outputting', 'ask_recording', 'ask_thinking'].includes(
            pipelineState,
          ) && (
            <span className="capsule-mode-picker-hint">
              {t('settings.polishModeNextRecording')}
            </span>
          )}
          <button
            type="button"
            aria-label={t('common.close')}
            onClick={() => close(false)}
            className="capsule-mode-picker-close"
          >
            <X size={19} />
          </button>
        </div>
        <PolishModeSelector inline onSelected={() => close(false)} />
      </div>
    </div>
  )
}
