import { afterEach, describe, expect, it } from 'vitest'
import { parseHash, parseHistoryDate } from '../router'

afterEach(() => {
  window.location.hash = ''
})

describe('desktop route parsing', () => {
  it('recognizes the input recovery route', () => {
    window.location.hash = '#/recovery'
    expect(parseHash()).toBe('recovery')
  })
  it('keeps direct recovery links inside Settings', () => {
    window.location.hash = '#/settings?pane=stt'
    expect(parseHash()).toBe('settings')
    window.location.hash = '#/settings?pane=llm'
    expect(parseHash()).toBe('settings')
  })

  it('recognizes a dated History link and rejects invalid calendar dates', () => {
    window.location.hash = '#/history?date=2026-09-30'
    expect(parseHash()).toBe('history')
    expect(parseHistoryDate()).toBe('2026-09-30')

    window.location.hash = '#/history?date=2026-09-31'
    expect(parseHistoryDate()).toBeNull()
    window.location.hash = '#/history?date=9999-99-99'
    expect(parseHistoryDate()).toBeNull()
  })
})
