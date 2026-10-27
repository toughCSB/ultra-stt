import { invoke } from '@tauri-apps/api/core'
import { isMacPlatform } from './accessibility'
import type { CapsuleSize } from './capsuleView'

export interface PhysicalPoint {
  x: number
  y: number
}

export interface PhysicalDimensions {
  width: number
  height: number
}

export interface CapsuleWorkArea {
  position: PhysicalPoint
  size: PhysicalDimensions
}

export interface CapsuleMonitorBounds {
  position: PhysicalPoint
  size: PhysicalDimensions
}

export interface CapsulePlacementMonitor {
  scaleFactor: number
  workArea: CapsuleWorkArea
}

export interface CapsuleMonitorGeometry extends CapsuleMonitorBounds, CapsulePlacementMonitor {}

export interface CapsuleWindowRect extends PhysicalPoint, PhysicalDimensions {}

/**
 * The native capsule window. Commands never fail: a window that cannot be moved or shown
 * is left as it is. Queries answer `null` when the platform cannot tell, except
 * `monitors()`, which rejects so callers can keep the current placement.
 */
export interface CapsuleWindow {
  show(): Promise<void>
  hide(): Promise<void>
  /** Puts the capsule above other windows without taking focus. */
  raise(): Promise<void>
  setFocusable(focusable: boolean): Promise<void>
  setSize(logical: CapsuleSize): Promise<void>
  /** `point` is in physical pixels on a monitor with `scaleFactor`. */
  setPosition(point: PhysicalPoint, scaleFactor: number): Promise<void>
  outerPosition(): Promise<PhysicalPoint | null>
  outerSize(): Promise<PhysicalDimensions | null>
  scaleFactor(): Promise<number>
  monitors(): Promise<CapsuleMonitorGeometry[]>
  currentMonitor(): Promise<CapsuleMonitorGeometry | null>
  primaryMonitor(): Promise<CapsuleMonitorGeometry | null>
  /** The monitor the user was working on when the recording started. */
  recordingMonitor(): Promise<CapsuleMonitorGeometry | null>
}

/** Puts the capsule above other windows without taking focus. */
export async function raiseCapsule(): Promise<void> {
  await invoke('raise_capsule').catch((error) =>
    console.warn('Could not raise recording capsule', error),
  )
}

/** The capsule's Tauri window. */
export async function tauriCapsuleWindow(): Promise<CapsuleWindow> {
  const {
    getCurrentWindow,
    LogicalSize,
    LogicalPosition,
    PhysicalPosition,
    availableMonitors,
    currentMonitor,
    primaryMonitor,
  } = await import('@tauri-apps/api/window')
  const win = getCurrentWindow()
  return {
    show: () => win.show().catch(() => {}),
    hide: () => win.hide().catch(() => {}),
    raise: raiseCapsule,
    setFocusable: (focusable) => win.setFocusable(focusable).catch(() => {}),
    setSize: ({ width, height }) => win.setSize(new LogicalSize(width, height)).catch(() => {}),
    // macOS positions windows in logical points; Windows and Linux in physical pixels.
    setPosition: (point, scale) =>
      win
        .setPosition(
          isMacPlatform()
            ? new LogicalPosition(point.x / scale, point.y / scale)
            : new PhysicalPosition(point.x, point.y),
        )
        .catch(() => {}),
    outerPosition: () => win.outerPosition().catch(() => null),
    outerSize: () => win.outerSize().catch(() => null),
    scaleFactor: () => win.scaleFactor().catch(() => 1),
    monitors: () => availableMonitors(),
    currentMonitor: () => currentMonitor().catch(() => null),
    primaryMonitor: () => primaryMonitor().catch(() => null),
    recordingMonitor: () =>
      invoke<CapsuleMonitorGeometry | null>('get_recording_monitor').catch(() => null),
  }
}

// ─── Placement ──────────────────────────────────────────────────────────

export function getRecordingTargetMonitor(
  target: CapsuleMonitorGeometry | null | undefined,
  monitors: CapsuleMonitorGeometry[],
): CapsuleMonitorGeometry | undefined {
  if (!target) return undefined
  return monitors.find(
    (monitor) =>
      monitor.position.x === target.position.x &&
      monitor.position.y === target.position.y &&
      monitor.size.width === target.size.width &&
      monitor.size.height === target.size.height &&
      monitor.scaleFactor === target.scaleFactor,
  )
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), Math.max(min, max))
}

export function getCapsuleCenterPosition(
  monitor: CapsulePlacementMonitor,
  logicalSize: CapsuleSize,
): PhysicalPoint {
  const physicalWidth = Math.round(logicalSize.width * monitor.scaleFactor)
  const physicalHeight = Math.round(logicalSize.height * monitor.scaleFactor)
  const { position, size } = monitor.workArea
  const centeredX = position.x + Math.round((size.width - physicalWidth) / 2)
  const centeredY = position.y + Math.round((size.height - physicalHeight) / 2)

  return {
    x: clamp(centeredX, position.x, position.x + size.width - physicalWidth),
    y: clamp(centeredY, position.y, position.y + size.height - physicalHeight),
  }
}

export function isCapsuleVisibleOnAnyMonitor(
  capsule: CapsuleWindowRect,
  monitors: CapsuleMonitorBounds[],
): boolean {
  return monitors.some(({ position, size }) => {
    const right = position.x + size.width
    const bottom = position.y + size.height
    const capsuleRight = capsule.x + capsule.width
    const capsuleBottom = capsule.y + capsule.height

    return (
      capsule.x < right &&
      capsuleRight > position.x &&
      capsule.y < bottom &&
      capsuleBottom > position.y
    )
  })
}

export function getCapsuleRecoveryPosition(
  capsule: CapsuleWindowRect,
  monitors: (CapsuleMonitorBounds & { scaleFactor?: number })[],
  recoveryMonitor: CapsulePlacementMonitor,
  logicalSize: CapsuleSize,
  logicalCoordinateScale?: number,
): PhysicalPoint | null {
  const visibilityRect = logicalCoordinateScale
    ? {
        x: capsule.x / logicalCoordinateScale,
        y: capsule.y / logicalCoordinateScale,
        width: capsule.width / logicalCoordinateScale,
        height: capsule.height / logicalCoordinateScale,
      }
    : capsule
  const visibilityMonitors = logicalCoordinateScale
    ? monitors.map((monitor) => {
        const scale = monitor.scaleFactor ?? 1
        return {
          position: { x: monitor.position.x / scale, y: monitor.position.y / scale },
          size: { width: monitor.size.width / scale, height: monitor.size.height / scale },
        }
      })
    : monitors
  if (monitors.length === 0 || isCapsuleVisibleOnAnyMonitor(visibilityRect, visibilityMonitors)) {
    return null
  }

  return getCapsuleCenterPosition(recoveryMonitor, logicalSize)
}

// ─── Sync ───────────────────────────────────────────────────────────────

/** What the native window should become. */
export interface CapsuleWindowTarget {
  /** Logical size of the native window. */
  size: CapsuleSize
  visible: boolean
  compact: boolean
  focusable: boolean
  /** The mode menu opened or closed since the last sync. */
  modeMenuToggled: boolean
  contextMenuOpen: boolean
  /** How long to let the exit animation play before hiding (0 for reduced motion). */
  exitMs: number
}

/** What a sync leaves behind for the next one. */
export interface CapsuleWindowMemory {
  initialized: boolean
  /**
   * Center on the recording monitor at the next sync that completes. Set when a new
   * recording starts; stays set if that sync is superseded.
   */
  recenterPending: boolean
  /** The capsule is shown on screen (the React side may animate it in). */
  shown: boolean
  /** Whether the shown capsule is the compact idle icon. */
  shownCompact: boolean
  /** When a running exit animation ends. */
  exitDeadline: number | null
  /** Native window size after the last completed resize. */
  size: CapsuleSize | null
}

export interface CapsuleWindowEvents {
  shown(compact: boolean): void
  hidden(): void
  /** The window is large enough for the context menu. */
  contextMenuReady(): void
}

class Superseded extends Error {}

/**
 * Brings the native window to `target`. A newer sync supersedes this one: `isCurrent`
 * turns false and this sync stops right after the window call in progress.
 */
export async function syncCapsuleWindow(
  win: CapsuleWindow,
  target: CapsuleWindowTarget,
  memory: CapsuleWindowMemory,
  events: CapsuleWindowEvents,
  isCurrent: () => boolean,
): Promise<void> {
  const step = async <T>(work: Promise<T>): Promise<T> => {
    const result = await work
    if (!isCurrent()) throw new Superseded()
    return result
  }
  try {
    if (!isCurrent()) return
    await syncSteps(win, target, memory, events, step)
  } catch (error) {
    if (!(error instanceof Superseded)) throw error
  }
}

type Step = <T>(work: Promise<T>) => Promise<T>

async function syncSteps(
  win: CapsuleWindow,
  target: CapsuleWindowTarget,
  memory: CapsuleWindowMemory,
  events: CapsuleWindowEvents,
  step: Step,
): Promise<void> {
  const show = async () => {
    await step(win.show())
    await step(win.raise())
    memory.shownCompact = target.compact
    memory.shown = true
    events.shown(target.compact)
  }
  const hide = async () => {
    memory.shown = false
    events.hidden()
    await step(win.hide())
  }

  // Let the exit animation finish before the window disappears or changes shape.
  if (
    (!target.visible || memory.shownCompact !== target.compact) &&
    memory.initialized &&
    (memory.shown || memory.exitDeadline !== null)
  ) {
    memory.exitDeadline ??= Date.now() + target.exitMs
    memory.shown = false
    events.hidden()
    await step(sleep(Math.max(0, memory.exitDeadline - Date.now())))
    await step(win.hide())
    memory.exitDeadline = null
    if (!target.visible) return
  }
  if (target.visible) memory.exitDeadline = null
  await step(win.setFocusable(target.focusable))

  if (!memory.initialized || memory.recenterPending) {
    await step(win.setSize(target.size))
    await keepIfMonitorsUnknown(async () => {
      const monitors = await step(win.monitors())
      const recording = target.visible ? await step(win.recordingMonitor()) : null
      const monitor =
        getRecordingTargetMonitor(recording, monitors) ??
        (await step(win.primaryMonitor())) ??
        (await step(win.currentMonitor())) ??
        monitors[0]
      if (monitor) {
        const position = getCapsuleCenterPosition(monitor, target.size)
        await step(win.setPosition(position, monitor.scaleFactor))
        await step(win.setSize(target.size))
        await step(win.setPosition(position, monitor.scaleFactor))
      }
    })
    if (target.visible) await show()
    else await hide()
    memory.initialized = true
    memory.recenterPending = false
    memory.size = target.size
    return
  }

  const previous = memory.size
  if (
    target.modeMenuToggled &&
    previous?.width === target.size.width &&
    previous.height === target.size.height
  ) {
    if (target.visible) await show()
    return
  }
  const position = previous ? await step(win.outerPosition()) : null
  if (previous && position) {
    // Resize around the current vertical center, then pull a lost window back on screen.
    const monitor = await step(win.currentMonitor())
    const scale = monitor?.scaleFactor ?? (await step(win.scaleFactor()))
    const oldSize = await step(win.outerSize())
    const oldHeight = oldSize?.height ?? Math.round(previous.height * scale)
    const physicalWidth = Math.round(target.size.width * scale)
    const physicalHeight = Math.round(target.size.height * scale)
    let next = {
      x: position.x,
      y: Math.round(position.y + oldHeight / 2 - physicalHeight / 2),
    }
    let positionScale = scale
    await step(win.setSize(target.size))
    await keepIfMonitorsUnknown(async () => {
      const monitors = await step(win.monitors())
      if (monitors.length === 0) return
      const recoveryMonitor = (await step(win.primaryMonitor())) ?? monitor ?? monitors[0]
      const recovered = getCapsuleRecoveryPosition(
        { ...next, width: physicalWidth, height: physicalHeight },
        monitors,
        recoveryMonitor,
        target.size,
        isMacPlatform() ? scale : undefined,
      )
      if (recovered) {
        next = recovered
        positionScale = recoveryMonitor.scaleFactor
      }
    })
    await step(win.setPosition(next, positionScale))
  } else {
    await step(win.setSize(target.size))
  }
  memory.size = target.size

  if (target.contextMenuOpen) events.contextMenuReady()
  if (target.visible) await show()
  else await hide()
}

/** Runs `work`; when the monitor list cannot be read, keeps the current placement. */
async function keepIfMonitorsUnknown(work: () => Promise<void>): Promise<void> {
  try {
    await work()
  } catch (error) {
    if (error instanceof Superseded) throw error
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}
