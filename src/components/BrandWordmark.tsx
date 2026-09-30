import { useTranslation } from 'react-i18next'
import appIconUrl from '../assets/brand/app-icon.png'

export function BrandMark({ size = 28 }: { size?: number }) {
  return (
    <img
      src={appIconUrl}
      alt=""
      aria-hidden="true"
      width={size}
      height={size}
      style={{ width: size, height: size }}
      className="shrink-0 select-none object-contain rounded-[22%]"
    />
  )
}

export function BrandWordmark({ compact = false }: { compact?: boolean }) {
  const { t } = useTranslation()
  return (
    <span
      role="img"
      aria-label={`${t('app.name')} — ${t('app.tagline')}`}
      className={`inline-flex items-center ${compact ? 'gap-2' : 'gap-2.5'}`}
    >
      <BrandMark size={compact ? 30 : 38} />
      <span className="flex flex-col leading-none min-w-0">
        <span
          aria-hidden="true"
          className={`font-black text-text-primary truncate ${compact ? 'text-[15px]' : 'text-[17px]'}`}
          style={{ letterSpacing: '-0.015em' }}
        >
          Ultra STT
        </span>
        <span
          aria-hidden="true"
          className={`font-semibold uppercase text-accent truncate ${compact ? 'text-[8px] mt-[3px]' : 'text-[9px] mt-[3px]'}`}
          style={{ letterSpacing: '0.22em' }}
        >
          Voice Keyboard
        </span>
      </span>
    </span>
  )
}
