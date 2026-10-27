import { useRef, useCallback, useEffect } from 'react'
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion'
import { useAppStore } from '../../stores/appStore'
import { useRecording } from '../../hooks/useRecording'
import { useCapsuleResize } from '../../hooks/useCapsuleResize'
import { useCapsuleView } from '../../hooks/useCapsuleView'
import { abortAskDictation, abortRecording, stopAskFlow } from '../../lib/tauri'
import { CapsuleIdle } from './CapsuleIdle'
import { CapsulePreparing } from './CapsulePreparing'
import { CapsuleRecording } from './CapsuleRecording'
import { CapsuleProcessing } from './CapsuleProcessing'
import { CapsulePolishing } from './CapsulePolishing'
import { CapsuleComplete } from './CapsuleComplete'
import { CapsuleError } from './CapsuleError'
import { CapsuleRecovery } from './CapsuleRecovery'
import { CapsuleContextMenu } from './CapsuleContextMenu'
import { CapsuleAskRecording } from './CapsuleAskRecording'
import { CapsuleAskThinking } from './CapsuleAskThinking'
import { TranslateTargetMenu } from './TranslateTargetChip'
import { PolishModeMenu } from './PolishModeMenu'
import { CAPSULE_ENTER_MS, CAPSULE_EXIT_MS } from '../../lib/capsuleMotion'
import type { CapsuleTone } from '../../lib/capsuleView'

const DRAG_THRESHOLD = 5

const TONE_CLASS: Record<CapsuleTone, string> = {
  error: 'jelly-capsule-error',
  idle: 'jelly-capsule text-[#8dffc9]',
  live: 'live-capsule text-white',
  active: 'jelly-capsule-active text-white',
}

export function Capsule() {
  const reducedMotion = useReducedMotion()
  const pipelineState = useAppStore((s) => s.pipelineState)
  const contextMenuOpen = useAppStore((s) => s.contextMenuOpen)
  const setContextMenuOpen = useAppStore((s) => s.setContextMenuOpen)
  const contextMenuReady = useAppStore((s) => s.contextMenuReady)
  const setContextMenuReady = useAppStore((s) => s.setContextMenuReady)
  const translationTargetMenuOpen = useAppStore((s) => s.translationTargetMenuOpen)
  const setTranslationTargetMenuOpen = useAppStore((s) => s.setTranslationTargetMenuOpen)
  const polishModeMenuOpen = useAppStore((s) => s.polishModeMenuOpen)
  const setPolishModeMenuOpen = useAppStore((s) => s.setPolishModeMenuOpen)
  const { stopRecording, isRecording } = useRecording()

  const dragStart = useRef<{ x: number; y: number } | null>(null)
  const isDragging = useRef(false)

  const { visible: capsuleVisible, ...capsuleShellSize } = useCapsuleResize()
  const { state: capsuleState, recoveryEntry, tone } = useCapsuleView()
  const cancel = useCallback(() => {
    const abort =
      pipelineState === 'ask_recording' || pipelineState === 'ask_thinking'
        ? abortAskDictation
        : abortRecording
    void abort().catch((error) => console.error('Failed to cancel voice input:', error))
  }, [pipelineState])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape' || event.repeat) return
      if (translationTargetMenuOpen) return
      event.preventDefault()
      if (polishModeMenuOpen) setPolishModeMenuOpen(false)
      else if (contextMenuOpen) {
        setContextMenuOpen(false)
        setContextMenuReady(false)
      } else if (pipelineState !== 'idle' && pipelineState !== 'outputting') cancel()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [
    cancel,
    contextMenuOpen,
    pipelineState,
    polishModeMenuOpen,
    setContextMenuOpen,
    setContextMenuReady,
    setPolishModeMenuOpen,
    translationTargetMenuOpen,
  ])

  const handlePointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return
    dragStart.current = { x: e.clientX, y: e.clientY }
    isDragging.current = false
  }, [])

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!dragStart.current || isDragging.current) return
    const dx = e.clientX - dragStart.current.x
    const dy = e.clientY - dragStart.current.y
    if (Math.abs(dx) > DRAG_THRESHOLD || Math.abs(dy) > DRAG_THRESHOLD) {
      isDragging.current = true
      dragStart.current = null
      import('@tauri-apps/api/window')
        .then(({ getCurrentWindow }) => {
          getCurrentWindow()
            .startDragging()
            .catch(() => {})
        })
        .catch(() => {})
    }
  }, [])

  const handlePointerUp = useCallback(
    (e: React.PointerEvent) => {
      if (e.button !== 0) return
      if (isDragging.current) {
        isDragging.current = false
        dragStart.current = null
        return
      }
      dragStart.current = null

      if (pipelineState === 'ask_recording') {
        void stopAskFlow().catch((error) => {
          console.error('Failed to stop Ask flow:', error)
        })
      } else if (isRecording) {
        stopRecording()
      }
    },
    [isRecording, pipelineState, stopRecording],
  )

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault()
    if (!contextMenuOpen) {
      setPolishModeMenuOpen(false)
      setTranslationTargetMenuOpen(false)
      setContextMenuOpen(true)
    }
  }

  const handleCloseMenu = () => {
    setContextMenuReady(false)
    setContextMenuOpen(false)
  }

  useEffect(() => {
    if (
      translationTargetMenuOpen &&
      (pipelineState !== 'recording' || capsuleState !== 'recording')
    ) {
      setTranslationTargetMenuOpen(false)
    }
  }, [capsuleState, pipelineState, setTranslationTargetMenuOpen, translationTargetMenuOpen])

  return (
    <div
      className="w-full h-full flex items-center justify-start relative"
      style={{ background: 'transparent' }}
      onContextMenu={handleContextMenu}
    >
      <AnimatePresence initial={false}>
        {capsuleVisible && (
          <motion.div
            key="capsule-shell"
            initial={reducedMotion ? false : { opacity: 0, y: 6, scale: 0.985 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{
              opacity: 0,
              y: reducedMotion ? 0 : 4,
              scale: reducedMotion ? 1 : 0.99,
              transition: {
                duration: reducedMotion ? 0 : CAPSULE_EXIT_MS / 1000,
                ease: [0.4, 0, 1, 1],
              },
            }}
            transition={{
              duration: reducedMotion ? 0 : CAPSULE_ENTER_MS / 1000,
              ease: [0.16, 1, 0.3, 1],
            }}
            className={`absolute left-3 rounded-full overflow-hidden pointer-events-auto shrink-0 ${TONE_CLASS[tone]}`}
            style={capsuleShellSize}
            onPointerDown={handlePointerDown}
            onPointerMove={handlePointerMove}
            onPointerUp={handlePointerUp}
          >
            <AnimatePresence mode="sync" initial={false}>
              <motion.div
                key={polishModeMenuOpen ? 'polish_modes' : capsuleState}
                className="absolute inset-0 capsule-content"
                initial={reducedMotion ? false : { opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: reducedMotion ? 0 : 0.08, ease: 'easeOut' }}
              >
                {polishModeMenuOpen ? (
                  <PolishModeMenu />
                ) : (
                  <>
                    {capsuleState === 'idle' && <CapsuleIdle />}
                    {capsuleState === 'preparing' && <CapsulePreparing />}
                    {capsuleState === 'recording' && <CapsuleRecording />}
                    {capsuleState === 'transcribing' && <CapsuleProcessing />}
                    {capsuleState === 'polishing' && <CapsulePolishing />}
                    {capsuleState === 'outputting' && <CapsuleComplete />}
                    {capsuleState === 'ask_recording' && <CapsuleAskRecording />}
                    {capsuleState === 'ask_thinking' && <CapsuleAskThinking />}
                    {capsuleState === 'error' && <CapsuleError />}
                    {capsuleState === 'recovery' && recoveryEntry && (
                      <CapsuleRecovery entry={recoveryEntry} />
                    )}
                  </>
                )}
              </motion.div>
            </AnimatePresence>
          </motion.div>
        )}
      </AnimatePresence>

      {contextMenuOpen && contextMenuReady && (
        <div className="ml-2">
          <CapsuleContextMenu onClose={handleCloseMenu} />
        </div>
      )}
      <TranslateTargetMenu />
    </div>
  )
}
