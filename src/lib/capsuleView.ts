import type { PipelineState, RecoveryEntry } from '../stores/appStore'

/** What the capsule (실행창) shows. Error and recovery override the session phase. */
export type CapsuleState = PipelineState | 'error' | 'recovery'

/** Colour family of the capsule shell. */
export type CapsuleTone = 'error' | 'idle' | 'live' | 'active'

export interface CapsuleSize {
  width: number
  height: number
}

export interface CapsuleViewInput {
  pipelineState: PipelineState
  pipelineError: string | null
  recoveryEntries: RecoveryEntry[]
  recoveryAlertId: string | null
  contextMenuOpen: boolean
  translationTargetMenuOpen: boolean
  polishModeMenuOpen: boolean
  capsuleExpanded: boolean
  capsuleAutoHide: boolean
  liveTyping: boolean
}

export interface CapsuleView {
  state: CapsuleState
  /** The failed dictation offered for recovery, when the capsule shows one. */
  recoveryEntry: RecoveryEntry | undefined
  tone: CapsuleTone
  /** The visible capsule. */
  shell: CapsuleSize
  /** The native window: the shell or an open menu, plus room for the shadow. */
  window: CapsuleSize
  /** Only the 36px idle icon. */
  compact: boolean
  visible: boolean
  /** Whether the window may take keyboard focus (only the mode menu needs it). */
  focusable: boolean
}

/** Margin around the shell inside the native window. */
const WINDOW_MARGIN = 24

export function deriveCapsuleView(input: CapsuleViewInput): CapsuleView {
  const menuOpen =
    input.contextMenuOpen || input.translationTargetMenuOpen || input.polishModeMenuOpen
  const recoveryEntry =
    input.pipelineState === 'idle' && !menuOpen
      ? input.recoveryEntries.find((entry) => entry.id === input.recoveryAlertId)
      : undefined
  const hasError = input.pipelineError !== null
  const hasRecovery = recoveryEntry !== undefined
  const state: CapsuleState =
    input.pipelineState === 'idle' && hasRecovery
      ? 'recovery'
      : hasError
        ? 'error'
        : input.pipelineState
  const tone: CapsuleTone =
    state === 'error' ? 'error' : state === 'idle' ? 'idle' : input.liveTyping ? 'live' : 'active'
  const shell = getCapsuleShellSize(
    input.pipelineState,
    input.capsuleExpanded,
    hasError,
    hasRecovery,
    input.polishModeMenuOpen,
  )
  const content = input.translationTargetMenuOpen
    ? { width: Math.max(shell.width, 360), height: 180 }
    : input.contextMenuOpen
      ? { width: Math.max(shell.width, 220), height: 220 }
      : shell

  return {
    state,
    recoveryEntry,
    tone,
    shell,
    window: { width: content.width + WINDOW_MARGIN, height: content.height + WINDOW_MARGIN },
    compact: shell.width === 36,
    visible: getCapsuleVisibility({
      capsuleAutoHide: input.capsuleAutoHide,
      contextMenuOpen: input.contextMenuOpen,
      translationTargetMenuOpen: input.translationTargetMenuOpen,
      polishModeMenuOpen: input.polishModeMenuOpen,
      capsuleExpanded: input.capsuleExpanded,
      hasError,
      hasRecovery,
      pipelineState: input.pipelineState,
    }),
    focusable: getCapsuleFocusable(input.polishModeMenuOpen),
  }
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

export function getCapsuleShellSize(
  state: PipelineState,
  expanded: boolean,
  hasError: boolean,
  hasRecovery: boolean,
  forceActive = false,
): CapsuleSize {
  if (state === 'idle' && !hasError && !hasRecovery && !expanded && !forceActive)
    return { width: 36, height: 36 }
  return { width: 560, height: expanded ? 140 : 112 }
}
