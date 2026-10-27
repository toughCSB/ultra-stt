import { useEffect, useRef, useState } from 'react'
import { useAppStore, type PipelineState } from '../stores/appStore'
import { CAPSULE_EXIT_MS } from '../lib/capsuleMotion'
import {
  raiseCapsule,
  syncCapsuleWindow,
  tauriCapsuleWindow,
  type CapsuleWindowMemory,
} from '../lib/capsuleWindow'
import { useCapsuleView } from './useCapsuleView'

/** Phases in which a new recording begins; entering one recenters the capsule. */
const STARTING_STATES: PipelineState[] = ['preparing', 'recording', 'ask_recording']

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
  const [nativeVisible, setNativeVisible] = useState(false)
  const [nativeCompact, setNativeCompact] = useState(true)
  const memory = useRef<CapsuleWindowMemory>({
    initialized: false,
    recenterPending: false,
    shown: false,
    shownCompact: true,
    exitDeadline: null,
    size: null,
  })
  const previousPipelineState = useRef<PipelineState>('idle')
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
    const modeMenuToggled = previousModeMenuOpen.current !== polishModeMenuOpen
    previousModeMenuOpen.current = polishModeMenuOpen
    if (
      STARTING_STATES.includes(pipelineState) &&
      !STARTING_STATES.includes(previousPipelineState.current)
    ) {
      memory.current.recenterPending = true
    }
    previousPipelineState.current = pipelineState
    let cancelled = false
    const isCurrent = () => !cancelled && effectGeneration.current === generation
    const reducedMotion = () =>
      window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false
    tauriCapsuleWindow()
      .then((win) =>
        syncCapsuleWindow(
          win,
          {
            size: { width: windowWidth, height: windowHeight },
            visible: shouldShow,
            compact,
            focusable,
            modeMenuToggled,
            contextMenuOpen,
            exitMs: reducedMotion() ? 0 : CAPSULE_EXIT_MS,
          },
          memory.current,
          {
            shown: (shownCompact) => {
              setNativeCompact(shownCompact)
              setNativeVisible(true)
            },
            hidden: () => setNativeVisible(false),
            contextMenuReady: () => setContextMenuReady(true),
          },
          isCurrent,
        ),
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
