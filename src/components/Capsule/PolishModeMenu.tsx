import { ChevronDown, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { polishModeLabelKey } from '../../lib/polishStyles'
import { PolishModeSelector } from '../PolishModeSelector'
import { togglePolishModeMenu } from '../../lib/capsulePolishMenu'

export function PolishModeChip({ left }: { left: number }) {
  const { t } = useTranslation()
  const style = useAppStore((state) => state.config.polish_style)
  const open = useAppStore((state) => state.polishModeMenuOpen)
  return (
    <button
      type="button"
      aria-label={t('settings.polishModeTitle')}
      title={t('settings.polishModeTitle')}
      aria-haspopup="dialog"
      aria-expanded={open}
      onPointerDown={(event) => event.stopPropagation()}
      onPointerUp={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation()
        togglePolishModeMenu()
      }}
      className="absolute z-20 flex h-7 w-[84px] items-center justify-center gap-1 rounded-full border border-accent/40 bg-[#05251b]/95 px-2 text-[10px] font-semibold text-[#8dffc9] shadow-sm hover:border-accent"
      style={{ left, top: open ? 16 : '50%', transform: open ? undefined : 'translateY(-50%)' }}
    >
      <span className="truncate">{t(polishModeLabelKey(style))}</span>
      <ChevronDown size={11} className="shrink-0" />
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
    <>
      <div className="fixed inset-0 z-30" onClick={() => close(false)} />
      <div
        role="dialog"
        aria-label={t('settings.polishModeTitle')}
        className="jelly-card absolute left-3 top-[56px] z-40 w-[300px] p-3"
        onPointerDown={(event) => event.stopPropagation()}
        onPointerUp={(event) => event.stopPropagation()}
      >
        <div className="mb-2 flex items-center justify-between">
          <span className="text-[12px] font-semibold">{t('settings.polishModeTitle')}</span>
          <button
            type="button"
            aria-label={t('common.close')}
            onClick={() => close(false)}
            className="rounded p-1 text-text-secondary hover:bg-bg-tertiary"
          >
            <X size={13} />
          </button>
        </div>
        <PolishModeSelector compact onSelected={() => close(false)} />
        {['transcribing', 'polishing', 'outputting', 'ask_recording', 'ask_thinking'].includes(
          pipelineState,
        ) && (
          <p className="mt-2 text-[10px] text-text-secondary">
            {t('settings.polishModeNextRecording')}
          </p>
        )}
      </div>
    </>
  )
}
