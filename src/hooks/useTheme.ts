import { useEffect } from 'react'
import { useAppStore } from '../stores/appStore'

export const PALETTES = ['github', 'purple', 'ocean', 'sunset'] as const
export type Palette = (typeof PALETTES)[number]
const PALETTE_KEY = 'ultra-stt-palette'

export function readPalette(): Palette {
  const saved = localStorage.getItem(PALETTE_KEY)
  return PALETTES.includes(saved as Palette) ? (saved as Palette) : 'github'
}

export function writePalette(palette: Palette) {
  localStorage.setItem(PALETTE_KEY, palette)
  window.dispatchEvent(new Event('ultra-stt-palette'))
}

export function useTheme() {
  const theme = useAppStore((s) => s.config.theme)

  useEffect(() => {
    const root = document.documentElement
    const apply = () => {
      const dark =
        theme === 'dark' ||
        (theme !== 'light' && window.matchMedia('(prefers-color-scheme: dark)').matches)
      root.classList.toggle('dark', dark)
      root.dataset.colorMode = dark ? 'dark' : 'light'
      root.dataset.theme = readPalette()
    }
    apply()
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => {
      if (theme === 'system') apply()
    }
    mq.addEventListener('change', onChange)
    window.addEventListener('ultra-stt-palette', apply)
    return () => {
      mq.removeEventListener('change', onChange)
      window.removeEventListener('ultra-stt-palette', apply)
    }
  }, [theme])
}
