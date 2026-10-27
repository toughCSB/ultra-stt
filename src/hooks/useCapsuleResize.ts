import { useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, type PipelineState } from '../stores/appStore'
import { isMacPlatform } from '../lib/accessibility'
import { CAPSULE_EXIT_MS } from '../lib/capsuleMotion'
import type { CapsuleSize } from '../lib/capsuleView'
import { useCapsuleView } from './useCapsuleView'

interface PhysicalPoint {
  x: number
  y: number
}

interface PhysicalDimensions {
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

async function raiseCapsule(): Promise<void> {
  await invoke('raise_capsule').catch((error) =>
    console.warn('Could not raise recording capsule', error),
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

export function useCapsuleResize() {
  const view = useCapsuleView()
  const pipelineState = useAppStore((s) => s.pipelineState)
  const capsuleExpanded = useAppStore((s) => s.capsuleExpanded)
  const pipelineError = useAppStore((s) => s.pipelineError)
  const contextMenuOpen = useAppStore((s) => s.contextMenuOpen)
  const translationTargetMenuOpen = useAppStore((s) => s.translationTargetMenuOpen)
  const polishModeMenuOpen = useAppStore((s) => s.polishModeMenuOpen)
  const setContextMenuReady = useAppStore((s) => s.setContextMenuReady)
  const capsuleAutoHide = useAppStore((s) => s.config.capsule_auto_hide)
  const initialized = useRef(false)
  const [nativeVisible, setNativeVisible] = useState(false)
  const [nativeCompact, setNativeCompact] = useState(true)
  const nativeVisibleRef = useRef(false)
  const nativeCompactRef = useRef(true)
  const exitDeadline = useRef<number | null>(null)
  const previousPipelineState = useRef<PipelineState>('idle')
  const resetPosition = useRef(false)
  const prevWindowSize = useRef<{ width: number; height: number } | null>(null)
  const previousModeMenuOpen = useRef(false)
  const effectGeneration = useRef(0)

  const hasError = pipelineError !== null
  const hasRecovery = view.recoveryEntry !== undefined
  const shellSize = view.shell
  const compact = view.compact
  const shouldShow = view.visible
  const focusable = view.focusable
  const windowWidth = view.window.width
  const windowHeight = view.window.height

  useEffect(() => {
    if (pipelineState === 'idle') return
    let pending = false
    const interval = setInterval(() => {
      if (pending) return
      pending = true
      void raiseCapsule().finally(() => {
        pending = false
      })
    }, 1000)
    return () => clearInterval(interval)
  }, [pipelineState])

  useEffect(() => {
    const generation = ++effectGeneration.current
    const modeMenuChanged = previousModeMenuOpen.current !== polishModeMenuOpen
    previousModeMenuOpen.current = polishModeMenuOpen
    const startingStates: PipelineState[] = ['preparing', 'recording', 'ask_recording']
    if (
      startingStates.includes(pipelineState) &&
      !startingStates.includes(previousPipelineState.current)
    ) {
      resetPosition.current = true
    }
    previousPipelineState.current = pipelineState
    let cancelled = false
    const isCurrent = () => !cancelled && effectGeneration.current === generation
    import('@tauri-apps/api/window')
      .then(
        async ({
          getCurrentWindow,
          LogicalSize,
          LogicalPosition,
          PhysicalPosition,
          availableMonitors,
          currentMonitor,
          primaryMonitor,
        }) => {
          if (!isCurrent()) return

          const win = getCurrentWindow()
          const windowPosition = (point: PhysicalPoint, scale: number) =>
            isMacPlatform()
              ? new LogicalPosition(point.x / scale, point.y / scale)
              : new PhysicalPosition(point.x, point.y)
          if (
            (!shouldShow || nativeCompactRef.current !== compact) &&
            initialized.current &&
            (nativeVisibleRef.current || exitDeadline.current !== null)
          ) {
            const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
            exitDeadline.current ??= Date.now() + (reduced ? 0 : CAPSULE_EXIT_MS)
            nativeVisibleRef.current = false
            setNativeVisible(false)
            await new Promise((resolve) =>
              setTimeout(resolve, Math.max(0, exitDeadline.current! - Date.now())),
            )
            if (!isCurrent()) return
            await win.hide().catch(() => {})
            if (isCurrent()) exitDeadline.current = null
            if (!isCurrent() || !shouldShow) return
          }
          if (shouldShow) exitDeadline.current = null
          if (!isCurrent()) return
          await win.setFocusable(focusable).catch(() => {})
          if (!isCurrent()) return

          if (!initialized.current || resetPosition.current) {
            if (!isCurrent()) return
            await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
            if (!isCurrent()) return

            try {
              const monitors = await availableMonitors()
              if (!isCurrent()) return

              const target = shouldShow
                ? await invoke<CapsuleMonitorGeometry | null>('get_recording_monitor').catch(
                    () => null,
                  )
                : null
              if (!isCurrent()) return
              let monitor =
                getRecordingTargetMonitor(target, monitors) ??
                (await primaryMonitor().catch(() => null))
              if (!isCurrent()) return

              if (!monitor) {
                monitor = await currentMonitor().catch(() => null)
                if (!isCurrent()) return
              }
              monitor ??= monitors[0]

              if (monitor) {
                const position = getCapsuleCenterPosition(monitor, {
                  width: windowWidth,
                  height: windowHeight,
                })
                if (!isCurrent()) return
                await win.setPosition(windowPosition(position, monitor.scaleFactor)).catch(() => {})
                if (!isCurrent()) return
                await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
                if (!isCurrent()) return
                await win.setPosition(windowPosition(position, monitor.scaleFactor)).catch(() => {})
                if (!isCurrent()) return
              }
            } catch {
              /* ignore – monitor info unavailable */
            }

            if (!isCurrent()) return
            if (shouldShow) {
              await win.show().catch(() => {})
              if (isCurrent()) await raiseCapsule()
              if (isCurrent()) {
                nativeCompactRef.current = compact
                setNativeCompact(compact)
                nativeVisibleRef.current = true
                setNativeVisible(true)
              }
            } else {
              nativeVisibleRef.current = false
              setNativeVisible(false)
              await win.hide().catch(() => {})
            }
            if (!isCurrent()) return

            initialized.current = true
            resetPosition.current = false
            if (!isCurrent()) return
            prevWindowSize.current = { width: windowWidth, height: windowHeight }
            return
          }

          const prev = prevWindowSize.current
          if (modeMenuChanged && prev?.width === windowWidth && prev.height === windowHeight) {
            if (shouldShow) {
              await win.show().catch(() => {})
              if (isCurrent()) await raiseCapsule()
              if (isCurrent()) {
                nativeCompactRef.current = compact
                setNativeCompact(compact)
                nativeVisibleRef.current = true
                setNativeVisible(true)
              }
            }
            return
          }
          if (prev) {
            const pos = await win.outerPosition().catch(() => null)
            if (!isCurrent()) return

            if (pos) {
              const monitor = await currentMonitor().catch(() => null)
              if (!isCurrent()) return

              let scale = monitor?.scaleFactor
              if (scale === undefined) {
                scale = await win.scaleFactor().catch(() => 1)
                if (!isCurrent()) return
              }

              const oldSize = await win.outerSize().catch(() => null)
              if (!isCurrent()) return

              const oldHeight = oldSize?.height ?? Math.round(prev.height * scale)
              const physicalWidth = Math.round(windowWidth * scale)
              const physicalHeight = Math.round(windowHeight * scale)
              let newX = pos.x
              let newY = Math.round(pos.y + oldHeight / 2 - physicalHeight / 2)
              let positionScale = scale

              if (!isCurrent()) return
              await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
              if (!isCurrent()) return

              try {
                const monitors = await availableMonitors()
                if (!isCurrent()) return

                const proposedRect = {
                  x: newX,
                  y: newY,
                  width: physicalWidth,
                  height: physicalHeight,
                }
                if (monitors.length > 0) {
                  const recoveryMonitor =
                    (await primaryMonitor().catch(() => null)) ?? monitor ?? monitors[0]
                  if (!isCurrent()) return

                  const recovered = getCapsuleRecoveryPosition(
                    proposedRect,
                    monitors,
                    recoveryMonitor,
                    {
                      width: windowWidth,
                      height: windowHeight,
                    },
                    isMacPlatform() ? scale : undefined,
                  )
                  if (recovered) {
                    newX = recovered.x
                    newY = recovered.y
                    positionScale = recoveryMonitor.scaleFactor
                  }
                }
              } catch {
                /* keep the previous position when monitor info is unavailable */
              }

              if (!isCurrent()) return
              await win
                .setPosition(windowPosition({ x: newX, y: newY }, positionScale))
                .catch(() => {})
              if (!isCurrent()) return
            } else {
              if (!isCurrent()) return
              await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
              if (!isCurrent()) return
            }
          } else {
            if (!isCurrent()) return
            await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
            if (!isCurrent()) return
          }

          if (!isCurrent()) return
          prevWindowSize.current = { width: windowWidth, height: windowHeight }

          // Signal that the window has finished resizing for context menu
          if (contextMenuOpen) {
            if (!isCurrent()) return
            setContextMenuReady(true)
          }

          if (!isCurrent()) return
          if (shouldShow) {
            await win.show().catch(() => {})
            if (isCurrent()) await raiseCapsule()
            if (isCurrent()) {
              nativeCompactRef.current = compact
              setNativeCompact(compact)
              nativeVisibleRef.current = true
              setNativeVisible(true)
            }
          } else {
            nativeVisibleRef.current = false
            setNativeVisible(false)
            await win.hide().catch(() => {})
          }
          if (!isCurrent()) return
        },
      )
      .catch(() => {})

    return () => {
      cancelled = true
    }
  }, [
    polishModeMenuOpen,
    pipelineState,
    capsuleExpanded,
    hasError,
    hasRecovery,
    contextMenuOpen,
    translationTargetMenuOpen,
    capsuleAutoHide,
    setContextMenuReady,
    compact,
    shouldShow,
    focusable,
    windowWidth,
    windowHeight,
  ])

  return { ...shellSize, visible: nativeVisible && shouldShow && nativeCompact === compact }
}
