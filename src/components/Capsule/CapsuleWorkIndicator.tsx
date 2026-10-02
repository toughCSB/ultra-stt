import { motion, useReducedMotion } from 'framer-motion'
import appIconUrl from '../../assets/brand/app-icon.png'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { polishModeLabelKey } from '../../lib/polishStyles'
import { togglePolishModeMenu } from '../../lib/capsulePolishMenu'

interface CapsuleWorkIndicatorProps {
  tone?: 'steady' | 'thinking'
}

/** The app mark sits at the front of every capsule state. */
export function CapsuleBrand() {
  const { t } = useTranslation()
  const style = useAppStore((state) => state.config.polish_style)
  return (
    <button type="button" aria-label={`${t('settings.polishModeTitle')}: ${t(polishModeLabelKey(style))}`} title={t('settings.polishModeTitle')}
      className="shrink-0 rounded-[22%]" onPointerDown={(event) => event.stopPropagation()} onPointerUp={(event) => event.stopPropagation()}
      onClick={(event) => { event.stopPropagation(); togglePolishModeMenu() }}>
    <img
      src={appIconUrl}
      alt=""
      aria-hidden="true"
      className="w-6 h-6 shrink-0 rounded-[22%] object-contain"
    />
    </button>
  )
}

export function CapsuleWorkIndicator({ tone = 'steady' }: CapsuleWorkIndicatorProps) {
  const reduced = useReducedMotion()
  const baseDelay = tone === 'thinking' ? 0.16 : 0.12

  return (
    <div className="flex items-center gap-[3px] flex-shrink-0" aria-hidden="true">
      {[0, 1, 2].map((i) => (
        <motion.span
          key={i}
          className="block w-[4px] h-[4px] rounded-full bg-white/80"
          animate={reduced ? undefined : { opacity: [0.35, 1, 0.35], scale: [0.86, 1.08, 0.86] }}
          transition={{
            repeat: Infinity,
            duration: tone === 'thinking' ? 1.05 : 0.95,
            delay: i * baseDelay,
            ease: 'easeInOut',
          }}
        />
      ))}
    </div>
  )
}
