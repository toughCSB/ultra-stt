import { describe, expect, it } from 'vitest'
import { deriveCapsuleView, type CapsuleViewInput } from '../capsuleView'
import type { RecoveryEntry } from '../../stores/appStore'

const failedInsert: RecoveryEntry = {
  id: 'r1',
  sessionId: 7,
  createdAt: '2026-10-27T10:00:00',
  text: '회의는 3시입니다.',
  appName: 'Slack',
  reason: 'target_changed',
  charsInserted: 0,
}

function view(overrides: Partial<CapsuleViewInput> = {}) {
  return deriveCapsuleView({
    pipelineState: 'idle',
    pipelineError: null,
    recoveryEntries: [],
    recoveryAlertId: null,
    contextMenuOpen: false,
    translationTargetMenuOpen: false,
    polishModeMenuOpen: false,
    capsuleExpanded: false,
    capsuleAutoHide: false,
    liveTyping: false,
    ...overrides,
  })
}

describe('deriveCapsuleView', () => {
  it('shows the session phase, with error and recovery taking over', () => {
    const cases: [Partial<CapsuleViewInput>, string, string][] = [
      [{}, 'idle', 'idle'],
      [{ pipelineState: 'recording' }, 'recording', 'active'],
      [{ pipelineState: 'recording', liveTyping: true }, 'recording', 'live'],
      [{ pipelineState: 'polishing', pipelineError: 'boom' }, 'error', 'error'],
      [{ recoveryEntries: [failedInsert], recoveryAlertId: 'r1' }, 'recovery', 'active'],
      [
        { recoveryEntries: [failedInsert], recoveryAlertId: 'r1', pipelineError: 'boom' },
        'recovery',
        'active',
      ],
      [{ recoveryEntries: [failedInsert], recoveryAlertId: 'other' }, 'idle', 'idle'],
    ]
    for (const [input, state, tone] of cases) {
      const result = view(input)
      expect([result.state, result.tone], JSON.stringify(input)).toEqual([state, tone])
    }
  })

  it('offers recovery only while idle with every menu closed', () => {
    const alert = { recoveryEntries: [failedInsert], recoveryAlertId: 'r1' }
    expect(view(alert).recoveryEntry).toBe(failedInsert)
    for (const blocker of [
      { pipelineState: 'recording' as const },
      { contextMenuOpen: true },
      { translationTargetMenuOpen: true },
      { polishModeMenuOpen: true },
    ]) {
      expect(view({ ...alert, ...blocker }).recoveryEntry, JSON.stringify(blocker)).toBe(undefined)
    }
  })

  it('sizes the window for the shell or the open menu, plus the shadow margin', () => {
    expect([view().shell, view().window, view().compact]).toEqual([
      { width: 36, height: 36 },
      { width: 60, height: 60 },
      true,
    ])
    expect(view({ pipelineState: 'recording' }).window).toEqual({ width: 584, height: 136 })
    expect(view({ capsuleExpanded: true }).window).toEqual({ width: 584, height: 164 })
    expect(view({ contextMenuOpen: true }).window).toEqual({ width: 244, height: 244 })
    expect(view({ pipelineState: 'recording', contextMenuOpen: true }).window).toEqual({
      width: 584,
      height: 244,
    })
    expect(view({ translationTargetMenuOpen: true }).window).toEqual({ width: 384, height: 204 })
    expect(view({ polishModeMenuOpen: true }).window).toEqual({ width: 584, height: 136 })
  })

  it('hides only an idle, quiet capsule when auto-hide is on, and focuses only the mode menu', () => {
    expect(view({ capsuleAutoHide: true }).visible).toBe(false)
    expect(view({ capsuleAutoHide: true, pipelineState: 'preparing' }).visible).toBe(true)
    expect(view({ capsuleAutoHide: true, pipelineError: 'boom' }).visible).toBe(true)
    expect(view({ capsuleAutoHide: false }).visible).toBe(true)
    expect(view({ polishModeMenuOpen: true }).focusable).toBe(true)
    expect(view({ contextMenuOpen: true }).focusable).toBe(false)
  })
})
