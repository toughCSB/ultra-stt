import { useMemo } from 'react'
import { useAppStore } from '../stores/appStore'
import { deriveCapsuleView, type CapsuleView } from '../lib/capsuleView'

/** The capsule's view, derived once from the store for the component and its window. */
export function useCapsuleView(): CapsuleView {
  const pipelineState = useAppStore((s) => s.pipelineState)
  const pipelineError = useAppStore((s) => s.pipelineError)
  const recoveryEntries = useAppStore((s) => s.recoveryEntries)
  const recoveryAlertId = useAppStore((s) => s.recoveryAlertId)
  const contextMenuOpen = useAppStore((s) => s.contextMenuOpen)
  const translationTargetMenuOpen = useAppStore((s) => s.translationTargetMenuOpen)
  const polishModeMenuOpen = useAppStore((s) => s.polishModeMenuOpen)
  const capsuleExpanded = useAppStore((s) => s.capsuleExpanded)
  const capsuleAutoHide = useAppStore((s) => s.config.capsule_auto_hide)
  const liveTyping = useAppStore((s) => s.liveTyping)

  return useMemo(
    () =>
      deriveCapsuleView({
        pipelineState,
        pipelineError,
        recoveryEntries,
        recoveryAlertId,
        contextMenuOpen,
        translationTargetMenuOpen,
        polishModeMenuOpen,
        capsuleExpanded,
        capsuleAutoHide,
        liveTyping,
      }),
    [
      pipelineState,
      pipelineError,
      recoveryEntries,
      recoveryAlertId,
      contextMenuOpen,
      translationTargetMenuOpen,
      polishModeMenuOpen,
      capsuleExpanded,
      capsuleAutoHide,
      liveTyping,
    ],
  )
}
