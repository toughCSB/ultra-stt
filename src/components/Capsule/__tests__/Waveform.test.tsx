import { act, cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { Waveform, getVoiceVisualLevel } from '../Waveform'
import { useAppStore } from '../../../stores/appStore'

vi.mock('framer-motion', () => ({ useReducedMotion: () => true }))

afterEach(() => {
  cleanup()
  useAppStore.setState({ audioVolume: 0 })
})

describe('voice waveform', () => {
  it('makes quiet microphone RMS visible without inventing speech for silence or invalid data', () => {
    expect(getVoiceVisualLevel(0)).toBe(0)
    expect(getVoiceVisualLevel(0.001)).toBe(0)
    expect(getVoiceVisualLevel(0.02)).toBeGreaterThan(0.3)
    expect(getVoiceVisualLevel(2)).toBe(1)
    expect(getVoiceVisualLevel(Number.NaN)).toBe(0)
    expect(getVoiceVisualLevel(Number.POSITIVE_INFINITY)).toBe(0)
  })

  it('still shows actual voice level with reduced motion, and cleans up its subscription', () => {
    const { unmount } = render(<Waveform />)
    const bar = screen.getByRole('img').querySelector('span')!
    const quiet = bar.style.transform
    act(() => useAppStore.setState({ audioVolume: 0.08 }))
    expect(bar.style.transform).not.toBe(quiet)
    act(() => useAppStore.setState({ audioVolume: 0 }))
    expect(bar.style.transform).toBe(quiet)
    unmount()
    act(() => useAppStore.setState({ audioVolume: 0.5 }))
    expect(bar.style.transform).toBe(quiet)
  })
})
