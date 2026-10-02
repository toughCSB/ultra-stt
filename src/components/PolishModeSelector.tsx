import { useState } from 'react'
import { Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import modeArt from '../assets/brand/polish-modes.png'
import { POLISH_MODES } from '../lib/polishStyles'
import { setPolishStyle } from '../lib/tauri'
import { useAppStore, type PolishStyle } from '../stores/appStore'

export function PolishModeSelector({
  compact = false,
  onSelected,
}: {
  compact?: boolean
  onSelected?: () => void
}) {
  const { t } = useTranslation()
  const current = useAppStore((state) => state.config.polish_style)
  const applyPatch = useAppStore((state) => state.applyPersistedConfigPatch)
  const [pending, setPending] = useState(false)
  const [error, setError] = useState(false)

  const select = async (style: PolishStyle) => {
    setPending(true)
    setError(false)
    try {
      const patch = await setPolishStyle(style)
      applyPatch(patch)
      onSelected?.()
    } catch {
      setError(true)
    } finally {
      setPending(false)
    }
  }

  return (
    <div>
      <div
        role="radiogroup"
        aria-label={t('settings.polishModeTitle')}
        className={compact ? 'space-y-1' : 'grid grid-cols-2 gap-2 sm:grid-cols-5'}
      >
        {POLISH_MODES.map((mode, index) => (
          <button
            key={mode.value}
            type="button"
            role="radio"
            aria-checked={current === mode.value}
            aria-label={t(mode.labelKey)}
            disabled={pending}
            tabIndex={current === mode.value ? 0 : -1}
            onClick={() => void select(mode.value)}
            onKeyDown={(event) => {
              const direction = ['ArrowRight', 'ArrowDown'].includes(event.key)
                ? 1
                : ['ArrowLeft', 'ArrowUp'].includes(event.key)
                  ? -1
                  : 0
              if (!direction) return
              event.preventDefault()
              const nextIndex = (index + direction + POLISH_MODES.length) % POLISH_MODES.length
              const buttons =
                event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>(
                  '[role="radio"]',
                )
              buttons?.[nextIndex]?.focus()
              void select(POLISH_MODES[nextIndex].value)
            }}
            className={`relative rounded-[12px] border text-left transition-colors disabled:opacity-60 ${compact ? 'flex w-full items-center gap-2 p-2' : 'flex min-w-0 flex-col items-center gap-2 p-3 text-center'} ${current === mode.value ? 'border-accent bg-accent/10 text-accent ring-1 ring-accent/30' : 'border-border bg-bg-secondary/70 text-text-primary hover:border-accent/50'} focus-visible:outline-2 focus-visible:outline-accent`}
          >
            <span
              aria-hidden="true"
              className={`block shrink-0 rounded-[10px] ${compact ? 'h-7 w-7' : 'h-14 w-14'}`}
              style={{
                backgroundImage: `url(${modeArt})`,
                backgroundSize: '300% 200%',
                backgroundPosition: mode.position,
              }}
            />
            <span className="min-w-0 flex-1">
              <span className={`block font-semibold ${compact ? 'text-[12px]' : 'text-[13px]'}`}>
                {t(mode.labelKey)}
              </span>
              <span className="mt-1 block text-[10px] leading-relaxed text-text-secondary">
                {t(mode.descriptionKey)}
              </span>
            </span>
            {current === mode.value && (
              <Check
                size={compact ? 14 : 12}
                aria-hidden="true"
                className={compact ? 'shrink-0' : 'absolute right-2 top-2'}
              />
            )}
          </button>
        ))}
      </div>
      {error && (
        <p role="alert" className="mt-2 text-[11px] text-error">
          {t('settings.polishModeError')}
        </p>
      )}
    </div>
  )
}
