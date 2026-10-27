import { describe, expect, it } from 'vitest'
import {
  syncCapsuleWindow,
  type CapsuleMonitorGeometry,
  type CapsuleWindow,
  type CapsuleWindowMemory,
  type CapsuleWindowTarget,
  type PhysicalPoint,
} from '../capsuleWindow'

const primary: CapsuleMonitorGeometry = {
  position: { x: 0, y: 0 },
  size: { width: 1920, height: 1080 },
  scaleFactor: 1,
  workArea: { position: { x: 0, y: 0 }, size: { width: 1920, height: 1040 } },
}
const laptop: CapsuleMonitorGeometry = {
  position: { x: 1920, y: 0 },
  size: { width: 2880, height: 1800 },
  scaleFactor: 2,
  workArea: { position: { x: 1920, y: 0 }, size: { width: 2880, height: 1760 } },
}

/** A window in memory that records every call. */
class FakeWindow implements CapsuleWindow {
  calls: string[] = []
  position: PhysicalPoint | null = { x: 100, y: 500 }
  height = 136
  monitorList: CapsuleMonitorGeometry[] | Error = [primary, laptop]
  recording: CapsuleMonitorGeometry | null = laptop
  /** Called after each call; lets a test supersede the sync mid-way. */
  afterCall: (call: string) => void = () => {}

  private log(call: string) {
    this.calls.push(call)
    this.afterCall(call)
  }
  async show() {
    this.log('show')
  }
  async hide() {
    this.log('hide')
  }
  async raise() {
    this.log('raise')
  }
  async setFocusable(focusable: boolean) {
    this.log(`focusable ${focusable}`)
  }
  async setSize({ width, height }: { width: number; height: number }) {
    this.log(`size ${width}x${height}`)
  }
  async setPosition(point: PhysicalPoint, scale: number) {
    this.log(`position ${point.x},${point.y} @${scale}`)
  }
  async outerPosition() {
    this.log('read position')
    return this.position
  }
  async outerSize() {
    this.log('read size')
    return { width: 584, height: this.height }
  }
  async scaleFactor() {
    this.log('read scale')
    return 1
  }
  async monitors() {
    this.log('read monitors')
    if (this.monitorList instanceof Error) throw this.monitorList
    return this.monitorList
  }
  async currentMonitor() {
    this.log('read current monitor')
    return primary
  }
  async primaryMonitor() {
    this.log('read primary monitor')
    return primary
  }
  async recordingMonitor() {
    this.log('read recording monitor')
    return this.recording
  }
}

function target(overrides: Partial<CapsuleWindowTarget> = {}): CapsuleWindowTarget {
  return {
    size: { width: 584, height: 136 },
    visible: true,
    compact: false,
    focusable: false,
    modeMenuToggled: false,
    contextMenuOpen: false,
    exitMs: 0,
    ...overrides,
  }
}

function memory(overrides: Partial<CapsuleWindowMemory> = {}): CapsuleWindowMemory {
  return {
    initialized: false,
    recenterPending: false,
    shown: false,
    shownCompact: true,
    exitDeadline: null,
    size: null,
    ...overrides,
  }
}

function recorder() {
  const events: string[] = []
  return {
    events,
    handlers: {
      shown: (compact: boolean) => events.push(`shown${compact ? ' compact' : ''}`),
      hidden: () => events.push('hidden'),
      contextMenuReady: () => events.push('menu ready'),
    },
  }
}

describe('syncCapsuleWindow', () => {
  it('centers a new recording on the monitor the user was working on, then shows it', async () => {
    const win = new FakeWindow()
    const state = memory({
      initialized: true,
      recenterPending: true,
      shown: true,
      shownCompact: false,
    })
    const { events, handlers } = recorder()

    await syncCapsuleWindow(win, target(), state, handlers, () => true)

    // 584x136 logical on the 2x laptop is 1168x272 physical, centered in its work area.
    expect(win.calls).toEqual([
      'focusable false',
      'size 584x136',
      'read monitors',
      'read recording monitor',
      'position 2776,744 @2',
      'size 584x136',
      'position 2776,744 @2',
      'show',
      'raise',
    ])
    expect(events).toEqual(['shown'])
    expect(state).toMatchObject({
      recenterPending: false,
      shown: true,
      size: { width: 584, height: 136 },
    })
  })

  it('stops when a newer sync takes over and recenters on the next one', async () => {
    const win = new FakeWindow()
    const state = memory({ initialized: true, recenterPending: true })
    let current = true
    win.afterCall = (call) => {
      if (call === 'read monitors') current = false
    }
    const { events, handlers } = recorder()

    await syncCapsuleWindow(win, target(), state, handlers, () => current)

    expect(win.calls).toEqual(['focusable false', 'size 584x136', 'read monitors'])
    expect(events).toEqual([])
    expect(state.recenterPending).toBe(true)
  })

  it('resizes around the current vertical center without moving sideways', async () => {
    const win = new FakeWindow()
    win.height = 60
    const state = memory({
      initialized: true,
      shown: true,
      shownCompact: false,
      size: { width: 584, height: 60 },
    })
    const { handlers } = recorder()

    await syncCapsuleWindow(win, target(), state, handlers, () => true)

    // Old center 500 + 60/2 = 530; the 136px window keeps it: 530 - 68 = 462.
    expect(win.calls).toContain('position 100,462 @1')
    expect(win.calls.slice(-2)).toEqual(['show', 'raise'])
  })

  it('pulls a window that ended up off every monitor back to the primary work area', async () => {
    const win = new FakeWindow()
    win.position = { x: -5000, y: -5000 }
    const state = memory({
      initialized: true,
      shown: true,
      shownCompact: false,
      size: { width: 584, height: 136 },
    })
    const { handlers } = recorder()

    await syncCapsuleWindow(win, target(), state, handlers, () => true)

    expect(win.calls).toContain('position 668,452 @1')
  })

  it('keeps the current placement when the monitor list cannot be read', async () => {
    const win = new FakeWindow()
    win.monitorList = new Error('no displays')
    const state = memory()
    const { events, handlers } = recorder()

    await syncCapsuleWindow(win, target(), state, handlers, () => true)

    expect(win.calls.some((call) => call.startsWith('position'))).toBe(false)
    expect(win.calls.slice(-2)).toEqual(['show', 'raise'])
    expect(events).toEqual(['shown'])
    expect(state.initialized).toBe(true)
  })

  it('lets the exit animation finish before hiding, and does nothing else', async () => {
    const win = new FakeWindow()
    const state = memory({ initialized: true, shown: true, size: { width: 60, height: 60 } })
    const { events, handlers } = recorder()

    await syncCapsuleWindow(win, target({ visible: false }), state, handlers, () => true)

    expect(events).toEqual(['hidden'])
    expect(win.calls).toEqual(['hide'])
    expect(state).toMatchObject({ shown: false, exitDeadline: null })
  })

  it('only reshows when the mode menu toggles without changing the window size', async () => {
    const win = new FakeWindow()
    const state = memory({
      initialized: true,
      shown: true,
      shownCompact: false,
      size: { width: 584, height: 136 },
    })
    const { handlers } = recorder()

    await syncCapsuleWindow(
      win,
      target({ modeMenuToggled: true, focusable: true }),
      state,
      handlers,
      () => true,
    )

    expect(win.calls).toEqual(['focusable true', 'show', 'raise'])
  })

  it('tells the context menu once the window has grown for it', async () => {
    const win = new FakeWindow()
    const state = memory({
      initialized: true,
      shown: true,
      shownCompact: false,
      size: { width: 584, height: 136 },
    })
    const { events, handlers } = recorder()

    await syncCapsuleWindow(
      win,
      target({ size: { width: 584, height: 244 }, contextMenuOpen: true }),
      state,
      handlers,
      () => true,
    )

    expect(events).toEqual(['menu ready', 'shown'])
  })
})
