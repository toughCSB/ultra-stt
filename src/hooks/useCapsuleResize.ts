import { useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore, type PipelineState } from '../stores/appStore'

interface CapsuleSize {
  width: number
  height: number
}

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

const CAPSULE_BOTTOM_MARGIN = 80

async function raiseCapsule(): Promise<void> {
  await invoke('raise_capsule').catch((error) =>
    console.warn('Could not raise recording capsule', error),
  )
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), Math.max(min, max))
}

export function getCapsuleBottomCenterPosition(
  monitor: CapsulePlacementMonitor,
  logicalSize: CapsuleSize,
): PhysicalPoint {
  const physicalWidth = Math.round(logicalSize.width * monitor.scaleFactor)
  const physicalHeight = Math.round(logicalSize.height * monitor.scaleFactor)
  const margin = Math.round(CAPSULE_BOTTOM_MARGIN * monitor.scaleFactor)
  const { position, size } = monitor.workArea
  const centeredX = position.x + Math.round((size.width - physicalWidth) / 2)
  const bottomY = position.y + size.height - physicalHeight - margin

  return {
    x: clamp(centeredX, position.x, position.x + size.width - physicalWidth),
    y: clamp(bottomY, position.y, position.y + size.height - physicalHeight),
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
  monitors: CapsuleMonitorBounds[],
  recoveryMonitor: CapsulePlacementMonitor,
  logicalSize: CapsuleSize,
): PhysicalPoint | null {
  if (monitors.length === 0 || isCapsuleVisibleOnAnyMonitor(capsule, monitors)) {
    return null
  }

  return getCapsuleBottomCenterPosition(recoveryMonitor, logicalSize)
}

export interface CapsuleVisibilityInput {
  capsuleAutoHide: boolean
  contextMenuOpen: boolean
  translationTargetMenuOpen?: boolean
  polishModeMenuOpen?: boolean
  capsuleExpanded: boolean
  hasError: boolean
  hasRecovery?: boolean
  pipelineState: PipelineState
}

export function getCapsuleVisibility({
  capsuleAutoHide,
  contextMenuOpen,
  translationTargetMenuOpen = false,
  polishModeMenuOpen = false,
  capsuleExpanded,
  hasError,
  hasRecovery = false,
  pipelineState,
}: CapsuleVisibilityInput): boolean {
  return (
    !capsuleAutoHide ||
    contextMenuOpen ||
    translationTargetMenuOpen ||
    polishModeMenuOpen ||
    capsuleExpanded ||
    hasError ||
    hasRecovery ||
    pipelineState !== 'idle'
  )
}

export function getCapsuleFocusable(menuOpen = false): boolean {
  return menuOpen
}

function getSizeForState(
  state: PipelineState,
  expanded: boolean,
  hasError: boolean,
  hasRecovery: boolean,
  contextMenuOpen: boolean,
  translationTargetMenuOpen = false,
): CapsuleSize {
  if (translationTargetMenuOpen) return { width: 360, height: 180 }
  if (contextMenuOpen) return { width: 220, height: 220 }
  if (state === 'idle' && hasRecovery) return { width: 304, height: 36 }
  if (hasError) return { width: 200, height: 36 }
  if (expanded) return { width: 220, height: 90 }
  switch (state) {
    case 'idle':
      return { width: 36, height: 36 }
    case 'preparing':
      return { width: 180, height: 36 }
    case 'recording':
      return { width: 228, height: 36 }
    case 'transcribing':
    case 'polishing':
      return { width: 200, height: 36 }
    case 'outputting':
      return { width: 144, height: 36 }
    case 'ask_recording':
    case 'ask_thinking':
      return { width: 168, height: 36 }
    default:
      return { width: 36, height: 36 }
  }
}

export function useCapsuleResize() {
  const pipelineState = useAppStore((s) => s.pipelineState)
  const capsuleExpanded = useAppStore((s) => s.capsuleExpanded)
  const pipelineError = useAppStore((s) => s.pipelineError)
  const recoveryAlertId = useAppStore((s) => s.recoveryAlertId)
  const recoveryEntries = useAppStore((s) => s.recoveryEntries)
  const contextMenuOpen = useAppStore((s) => s.contextMenuOpen)
  const translationTargetMenuOpen = useAppStore((s) => s.translationTargetMenuOpen)
  const polishModeMenuOpen = useAppStore((s) => s.polishModeMenuOpen)
  const setContextMenuReady = useAppStore((s) => s.setContextMenuReady)
  const capsuleAutoHide = useAppStore((s) => s.config.capsule_auto_hide)
  const initialized = useRef(false)
  const previousPipelineState = useRef<PipelineState>('idle')
  const resetPosition = useRef(false)
  const prevWindowSize = useRef<{ width: number; height: number } | null>(null)
  const modeMenuAnchor = useRef<PhysicalPoint | null>(null)
  const effectGeneration = useRef(0)

  const hasError = pipelineError !== null
  const hasRecovery =
    pipelineState === 'idle' && recoveryEntries.some((entry) => entry.id === recoveryAlertId)

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
    const startingStates: PipelineState[] = ['preparing', 'recording', 'ask_recording']
    if (
      startingStates.includes(pipelineState) &&
      !startingStates.includes(previousPipelineState.current)
    ) {
      resetPosition.current = true
      modeMenuAnchor.current = null
    }
    previousPipelineState.current = pipelineState
    let cancelled = false
    const isCurrent = () => !cancelled && effectGeneration.current === generation
    const size = polishModeMenuOpen
      ? { width: 324, height: 420 }
      : getSizeForState(
          pipelineState,
          capsuleExpanded,
          hasError,
          hasRecovery,
          contextMenuOpen,
          translationTargetMenuOpen,
        )
    if (
      !polishModeMenuOpen &&
      !contextMenuOpen &&
      !translationTargetMenuOpen &&
      pipelineState !== 'idle'
    ) {
      size.width += ['transcribing', 'polishing', 'ask_thinking'].includes(pipelineState) ? 130 : 90
    }
    const windowWidth = size.width + 24
    const windowHeight = size.height + 24
    const shouldShow = getCapsuleVisibility({
      capsuleAutoHide,
      contextMenuOpen,
      translationTargetMenuOpen,
      polishModeMenuOpen,
      capsuleExpanded,
      hasError,
      hasRecovery,
      pipelineState,
    })

    import('@tauri-apps/api/window')
      .then(
        async ({
          getCurrentWindow,
          LogicalSize,
          PhysicalPosition,
          availableMonitors,
          currentMonitor,
          primaryMonitor,
        }) => {
          if (!isCurrent()) return

          const win = getCurrentWindow()
          if (!isCurrent()) return
          await win.setFocusable(getCapsuleFocusable(polishModeMenuOpen)).catch(() => {})
          if (!isCurrent()) return

          if (!initialized.current || resetPosition.current) {
            if (!isCurrent()) return
            await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
            if (!isCurrent()) return

            try {
              const monitors = await availableMonitors()
              if (!isCurrent()) return

              let monitor = await primaryMonitor().catch(() => null)
              if (!isCurrent()) return

              if (!monitor) {
                monitor = await currentMonitor().catch(() => null)
                if (!isCurrent()) return
              }
              monitor ??= monitors[0]

              if (monitor) {
                const position = getCapsuleBottomCenterPosition(monitor, {
                  width: windowWidth,
                  height: windowHeight,
                })
                if (!isCurrent()) return
                await win.setPosition(new PhysicalPosition(position.x, position.y)).catch(() => {})
                if (!isCurrent()) return
                await win.setSize(new LogicalSize(windowWidth, windowHeight)).catch(() => {})
                if (!isCurrent()) return
              }
            } catch {
              /* ignore – monitor info unavailable */
            }

            if (!isCurrent()) return
            if (shouldShow) {
              await win.show().catch(() => {})
              if (isCurrent()) await raiseCapsule()
            } else {
              await win.hide().catch(() => {})
            }
            if (!isCurrent()) return

            initialized.current = true
            resetPosition.current = false
            if (!isCurrent()) return
            prevWindowSize.current = { width: windowWidth, height: windowHeight }
            return
          }

          // Subsequent resizes: left edge + vertical center stay fixed.
          // Since content is always padded 12px each side, the capsule at x=12
          // is identical to a centered capsule — so the mic icon never moves.
          const prev = prevWindowSize.current
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
              if (polishModeMenuOpen) {
                modeMenuAnchor.current ??= pos
                newY = Math.round(pos.y + oldHeight - physicalHeight)
                if (monitor) {
                  newX = clamp(
                    newX,
                    monitor.workArea.position.x,
                    monitor.workArea.position.x + monitor.workArea.size.width - physicalWidth,
                  )
                  newY = clamp(
                    newY,
                    monitor.workArea.position.y,
                    monitor.workArea.position.y + monitor.workArea.size.height - physicalHeight,
                  )
                }
              } else if (modeMenuAnchor.current) {
                newX = modeMenuAnchor.current.x
                newY = modeMenuAnchor.current.y
                modeMenuAnchor.current = null
              }

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
                  )
                  if (recovered) {
                    newX = recovered.x
                    newY = recovered.y
                  }
                }
              } catch {
                /* keep the previous position when monitor info is unavailable */
              }

              if (!isCurrent()) return
              await win.setPosition(new PhysicalPosition(newX, newY)).catch(() => {})
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
          } else {
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
  ])

  return getSizeForState(
    pipelineState,
    capsuleExpanded,
    hasError,
    hasRecovery,
    contextMenuOpen,
    translationTargetMenuOpen,
  )
}
