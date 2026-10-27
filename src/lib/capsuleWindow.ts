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
