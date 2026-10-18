import { useEffect, useRef } from 'react'
import { useReducedMotion } from 'framer-motion'
import { useAppStore } from '../../stores/appStore'

const BAR_COUNT = 28
const MIN_SCALE = 0.12
const BAR_PROFILE = Array.from({ length: BAR_COUNT }, (_, index) => {
  const center = (BAR_COUNT - 1) / 2
  return 0.48 + 0.52 * (1 - Math.abs(index - center) / center)
})

export function getVoiceVisualLevel(volume: number): number {
  return Number.isFinite(volume) ? Math.min(1, Math.sqrt(Math.max(0, volume - 0.002)) * 2.4) : 0
}

export function Waveform() {
  const barsRef = useRef<(HTMLSpanElement | null)[]>([])
  const rafRef = useRef<number>(0)
  const smoothedVolume = useRef(0)
  const reduced = useReducedMotion()

  useEffect(() => {
    if (reduced) {
      const updateLevel = () => {
        const level = getVoiceVisualLevel(useAppStore.getState().audioVolume)
        barsRef.current.forEach((bar, index) => {
          if (!bar) return
          bar.style.transform = `scaleY(${MIN_SCALE + level * BAR_PROFILE[index] * 0.88})`
          bar.style.opacity = `${0.46 + level * 0.54}`
        })
      }
      updateLevel()
      return useAppStore.subscribe((state, previous) => {
        if (state.audioVolume !== previous.audioVolume) updateLevel()
      })
    }

    const animate = () => {
      const volume = getVoiceVisualLevel(useAppStore.getState().audioVolume)
      smoothedVolume.current +=
        (volume - smoothedVolume.current) * (volume > smoothedVolume.current ? 0.38 : 0.14)
      const now = performance.now()
      barsRef.current.forEach((bar, index) => {
        if (!bar) return
        const ripple = 0.72 + 0.28 * Math.sin(now / 125 - index * 0.54)
        const amplitude = Math.min(
          1,
          MIN_SCALE + smoothedVolume.current * BAR_PROFILE[index] * ripple * 1.55,
        )
        bar.style.transform = `scaleY(${amplitude})`
        bar.style.opacity = `${0.46 + amplitude * 0.54}`
      })
      rafRef.current = requestAnimationFrame(animate)
    }

    rafRef.current = requestAnimationFrame(animate)
    return () => cancelAnimationFrame(rafRef.current)
  }, [reduced])

  return (
    <div className="capsule-waveform" role="img" aria-label="Audio level">
      {BAR_PROFILE.map((_, index) => (
        <span
          key={index}
          ref={(el) => {
            barsRef.current[index] = el
          }}
          className="capsule-waveform-bar"
        />
      ))}
    </div>
  )
}
