import { useTranslation } from 'react-i18next'
import { motion, useReducedMotion } from 'framer-motion'
import { Zap } from 'lucide-react'
import { abortRecording, switchRecordingMode } from '../../lib/tauri'
import { CapsuleBrand } from './CapsuleWorkIndicator'
import { Waveform } from './Waveform'
import { DurationTimer } from './DurationTimer'
import { TranslateTargetChip } from './TranslateTargetChip'
import { useAppStore } from '../../stores/appStore'

export function CapsuleRecording() {
  const { t } = useTranslation()
  const reduced = useReducedMotion()
  const liveTyping = useAppStore((state) => state.liveTyping)

  const handleCancel = async (e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      await abortRecording()
    } catch (err) {
      console.error('Failed to abort recording:', err)
    }
  }

  const handleSwitchMode = async (e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      await switchRecordingMode(!liveTyping)
    } catch (err) {
      console.error('Failed to switch recording mode:', err)
    }
  }

  const stopPointerPropagation = (e: React.PointerEvent) => {
    e.stopPropagation()
  }

  if (liveTyping) {
    return (
      <motion.div className="relative z-10 flex items-center gap-2 h-9 px-3">
        <motion.span
          className="flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full bg-white text-[#c2410c]"
          animate={reduced ? undefined : { scale: [1, 1.12, 1] }}
          transition={{ repeat: Infinity, duration: 1.2, ease: 'easeInOut' }}
        >
          <Zap size={14} strokeWidth={2.6} fill="currentColor" />
        </motion.span>
        <span className="flex-shrink-0 text-[13px] font-extrabold tracking-tight text-white">
          {t('capsule.liveTyping')}
        </span>
        <Waveform />
        <div className="flex-1" />
        <button
          onPointerDown={stopPointerPropagation}
          onPointerUp={stopPointerPropagation}
          onClick={handleSwitchMode}
          title={t('capsule.switchToStt')}
          aria-label={t('capsule.switchToStt')}
          className="flex-shrink-0 rounded-full border border-white/60 bg-white/15 px-1.5 py-0.5 text-[9px] font-bold text-white hover:bg-white/30 cursor-pointer"
        >
          STT
        </button>
        <DurationTimer />
        <button
          onPointerDown={stopPointerPropagation}
          onPointerUp={stopPointerPropagation}
          onClick={handleCancel}
          aria-label={t('capsule.cancelRecording')}
          className="flex-shrink-0 p-1 rounded-full text-white/80 hover:text-white hover:bg-white/20 transition-colors bg-transparent border-none cursor-pointer"
        >
          <span className="text-[9px] font-semibold">Esc</span>
        </button>
      </motion.div>
    )
  }

  return (
    <motion.div className="relative z-10 flex items-center gap-2 h-9 px-3">
      <CapsuleBrand />
      {/* White pulse dot — gentle opacity loop */}
      <motion.div
        className="w-2 h-2 rounded-full bg-white/80 flex-shrink-0"
        animate={reduced ? undefined : { opacity: [1, 0.5, 1] }}
        transition={{ repeat: Infinity, duration: 1.5, ease: 'easeInOut' }}
      />
      <Waveform />
      <TranslateTargetChip />
      <div className="flex-1" />
      <button
        onPointerDown={stopPointerPropagation}
        onPointerUp={stopPointerPropagation}
        onClick={handleSwitchMode}
        title={t('capsule.switchToLive')}
        aria-label={t('capsule.switchToLive')}
        className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-full bg-orange-500 text-white hover:bg-orange-400 cursor-pointer border-none"
      >
        <Zap size={11} strokeWidth={2.6} fill="currentColor" />
      </button>
      <DurationTimer />
      <button
        onPointerDown={stopPointerPropagation}
        onPointerUp={stopPointerPropagation}
        onClick={handleCancel}
        aria-label={t('capsule.cancelRecording')}
        className="flex-shrink-0 p-1 rounded-full text-white/70 hover:text-white hover:bg-white/15 transition-colors bg-transparent border-none cursor-pointer"
      >
        <span className="text-[9px] font-semibold">Esc</span>
      </button>
    </motion.div>
  )
}
