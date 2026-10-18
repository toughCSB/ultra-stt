import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { afterEach, describe, expect, it } from 'vitest'

const stylesheet = readFileSync(resolve('src/styles/globals.css'), 'utf8')
const canvasStyles = stylesheet.slice(
  stylesheet.indexOf('/* ─── Terminal canvas:'),
  stylesheet.indexOf('/* ─── Panels:'),
)

afterEach(() => {
  document.head.querySelectorAll('style').forEach((style) => style.remove())
  document.documentElement.className = ''
  delete document.documentElement.dataset.window
  document.body.querySelectorAll('[data-capsule-test]').forEach((element) => element.remove())
})

describe('window canvas background', () => {
  it('uses emerald, sapphire and crimson accents with large round controls', () => {
    document.documentElement.dataset.window = 'capsule'
    const style = document.createElement('style')
    style.textContent = stylesheet.slice(
      stylesheet.indexOf('/* ─── Panels:'),
      stylesheet.indexOf('/* ─── Scrollbar ─── */'),
    )
    document.head.appendChild(style)
    const accents = ['jelly-capsule-active', 'live-capsule', 'jelly-capsule-error'].map((name) => {
      const capsule = document.createElement('div')
      capsule.className = name
      capsule.dataset.capsuleTest = ''
      document.body.appendChild(capsule)
      return getComputedStyle(capsule).getPropertyValue('--capsule-accent')
    })
    expect(accents).toEqual(['#00e676', '#38bdf8', '#ff4d4d'])
    for (const className of ['capsule-action', 'capsule-mode-chip']) {
      const button = document.createElement('button')
      button.className = className
      button.dataset.capsuleTest = ''
      document.body.appendChild(button)
      expect(Number.parseFloat(getComputedStyle(button).height)).toBeGreaterThanOrEqual(44)
    }
    const brand = document.createElement('button')
    brand.className = 'capsule-brand'
    brand.dataset.capsuleTest = ''
    document.body.appendChild(brand)
    expect(getComputedStyle(brand).borderRadius).toBe('50%')
    expect(getComputedStyle(brand).overflow).toBe('hidden')
    for (const className of ['capsule-status-label', 'capsule-state-badge', 'capsule-mode-chip']) {
      const label = document.createElement('span')
      label.className = className
      label.textContent = '원문 · Original'
      label.dataset.capsuleTest = ''
      document.body.appendChild(label)
      expect(getComputedStyle(label).whiteSpace).toBe('nowrap')
    }
  })
  it('positions capsule menus independently of the shared relative card style', () => {
    document.documentElement.dataset.window = 'capsule'
    const style = document.createElement('style')
    style.textContent = stylesheet.slice(
      stylesheet.indexOf('/* ─── Panels:'),
      stylesheet.indexOf('/* ─── Scrollbar ─── */'),
    )
    document.head.appendChild(style)
    for (const name of ['capsule-mode-menu', 'capsule-translation-menu']) {
      const menu = document.createElement('div')
      menu.className = `jelly-card ${name}`
      menu.dataset.capsuleTest = ''
      document.body.appendChild(menu)
      expect(getComputedStyle(menu).position).toBe('absolute')
    }
  })
  it('preserves legible state and waveform colors instead of blending all content', () => {
    const style = document.createElement('style')
    style.textContent = stylesheet.slice(
      stylesheet.indexOf("html[data-window='capsule'] .jelly-capsule,"),
      stylesheet.indexOf('/* ─── Scrollbar ─── */'),
    )
    document.head.appendChild(style)
    const content = document.createElement('div')
    content.className = 'capsule-content'
    content.dataset.capsuleTest = ''
    document.body.appendChild(content)

    expect(getComputedStyle(content).mixBlendMode).toBe('normal')
  })

  it.each(['jelly-capsule', 'jelly-capsule-active', 'live-capsule', 'jelly-capsule-error'])(
    'keeps the approved %s hairline rounded without a rectangular outline',
    (className) => {
      document.documentElement.dataset.window = 'capsule'
      const style = document.createElement('style')
      style.textContent = stylesheet.slice(
        stylesheet.indexOf("html[data-window='capsule'] .jelly-capsule,"),
        stylesheet.indexOf('/* ─── Scrollbar ─── */'),
      )
      document.head.appendChild(style)
      const capsule = document.createElement('div')
      capsule.className = className
      capsule.dataset.capsuleTest = ''
      document.body.appendChild(capsule)

      const computed = getComputedStyle(capsule)
      expect(computed.borderTopStyle).toBe('solid')
      expect(computed.borderTopWidth).toBe('1px')
      expect(computed.borderRadius).toBe('9999px')
      expect(computed.outline).toBe('none')
      expect(computed.boxShadow).toContain('inset')
    },
  )

  it.each(['', 'dark'])('keeps the capsule transparent in %s mode', (theme) => {
    document.documentElement.className = theme
    const style = document.createElement('style')
    style.textContent = canvasStyles
    document.head.appendChild(style)

    for (const element of [document.documentElement, document.body]) {
      const computed = getComputedStyle(element)
      expect(['', 'none']).toContain(computed.backgroundImage)
      expect(computed.backgroundColor).toBe('rgba(0, 0, 0, 0)')
    }
  })

  it.each(['app-canvas', 'app-canvas dark'])('paints the main %s window', (classes) => {
    document.documentElement.className = classes
    const style = document.createElement('style')
    style.textContent = canvasStyles
    document.head.appendChild(style)

    expect(getComputedStyle(document.documentElement).backgroundImage).toContain('data:image/')
    expect(getComputedStyle(document.body).backgroundImage).toContain('data:image/')
  })
})
