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
  document.body.querySelectorAll('[data-capsule-test]').forEach((element) => element.remove())
})

describe('window canvas background', () => {
  it('blends the original icon black background out of the capsule content', () => {
    const style = document.createElement('style')
    style.textContent = stylesheet.slice(
      stylesheet.indexOf('.jelly-capsule {'),
      stylesheet.indexOf('/* ─── Scrollbar ─── */'),
    )
    document.head.appendChild(style)
    const content = document.createElement('div')
    content.className = 'capsule-content'
    content.dataset.capsuleTest = ''
    document.body.appendChild(content)

    expect(getComputedStyle(content).mixBlendMode).toBe('screen')
  })

  it.each(['jelly-capsule', 'jelly-capsule-active', 'jelly-capsule-error'])(
    'does not draw a border or shadow around %s',
    (className) => {
      const style = document.createElement('style')
      style.textContent = stylesheet.slice(
        stylesheet.indexOf('.jelly-capsule {'),
        stylesheet.indexOf('/* ─── Scrollbar ─── */'),
      )
      document.head.appendChild(style)
      const capsule = document.createElement('div')
      capsule.className = className
      capsule.dataset.capsuleTest = ''
      document.body.appendChild(capsule)

      const computed = getComputedStyle(capsule)
      expect(['', 'none']).toContain(computed.borderTopStyle)
      expect(computed.outline).toBe('none')
      expect(computed.boxShadow).toBe('none')
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
