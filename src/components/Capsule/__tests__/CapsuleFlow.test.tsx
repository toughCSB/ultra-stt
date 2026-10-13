import React from 'react'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '../../../stores/appStore'
import {
  abortAskDictation,
  abortRecording,
  copyRecoveryEntry,
  setActiveTranslationTarget,
  setPolishStyle,
  stopAskFlow,
  switchRecordingMode,
} from '../../../lib/tauri'
import { Capsule } from '../index'

const dragWindow = vi.hoisted(() => vi.fn().mockResolvedValue(undefined))
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ startDragging: dragWindow }),
}))

vi.mock('framer-motion', () => ({
  AnimatePresence: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  motion: new Proxy(
    {},
    {
      get:
        (_target, tag: string) =>
        ({ children, ...props }: React.HTMLAttributes<HTMLElement>) =>
          React.createElement(tag, props, children),
    },
  ),
  useReducedMotion: () => true,
}))

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}))

vi.mock('../../../hooks/useCapsuleResize', () => ({
  useCapsuleResize: () => ({ width: 200, height: 36 }),
}))

vi.mock('../../../lib/tauri', () => ({
  abortAskDictation: vi.fn().mockResolvedValue(undefined),
  abortRecording: vi.fn().mockResolvedValue(undefined),
  setActiveTranslationTarget: vi.fn().mockResolvedValue({
    targets: ['en', 'ja'],
    active_target: 'ja',
  }),
  setCapsuleAutoHide: vi.fn().mockResolvedValue(undefined),
  stopAskFlow: vi.fn().mockResolvedValue(undefined),
  copyRecoveryEntry: vi.fn().mockResolvedValue(undefined),
  setPolishStyle: vi.fn().mockResolvedValue({ polish_style: 'original', polish_enabled: true }),
  switchRecordingMode: vi.fn().mockResolvedValue(undefined),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  useAppStore.setState(useAppStore.getInitialState())
})

describe('Capsule flow states', () => {
  it('drags the recording shell without treating release as a stop click', async () => {
    useAppStore.setState({ pipelineState: 'recording' })
    const { container } = render(<Capsule />)
    const shell = container.querySelector('.jelly-capsule-active')!
    for (const [type, clientX] of [
      ['pointerdown', 20],
      ['pointermove', 40],
      ['pointerup', 40],
    ] as const) {
      const event = new Event(type, { bubbles: true })
      Object.defineProperties(event, {
        button: { value: 0 },
        clientX: { value: clientX },
        clientY: { value: 20 },
      })
      fireEvent(shell, event)
    }
    await waitFor(() => expect(dragWindow).toHaveBeenCalledTimes(1))
    expect(invoke).not.toHaveBeenCalledWith('stop_recording')
    expect(stopAskFlow).not.toHaveBeenCalled()
  })

  beforeEach(() => {
    useAppStore.setState({
      pipelineState: 'idle',
      pipelineError: null,
      contextMenuOpen: false,
      contextMenuReady: false,
      translationTargetMenuOpen: false,
      activeVoiceMode: null,
      partialTranscript: '',
    })
  })

  it('renders one centered original brand mark in the idle shell', () => {
    const { container } = render(<Capsule />)
    const shell = container.querySelector('.jelly-capsule')

    expect(shell?.querySelectorAll('img')).toHaveLength(1)
    expect(shell?.querySelectorAll('svg')).toHaveLength(0)
  })

  it('keeps a failed finished sentence available from the idle capsule without stopping capture', async () => {
    useAppStore.setState({
      recoveryEntries: [
        {
          id: 'recover-2',
          sessionId: 2,
          createdAt: '2026-10-03T00:00:00Z',
          text: 'Complete sentence',
          appName: 'Editor',
          reason: 'output_failed',
          charsInserted: 0,
        },
      ],
      recoveryAlertId: 'recover-2',
    })
    render(<Capsule />)
    expect(screen.getByText(/Complete sentence/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'recovery.copy' }))
    await waitFor(() => expect(copyRecoveryEntry).toHaveBeenCalledWith('recover-2'))
    expect(stopAskFlow).not.toHaveBeenCalled()
    expect(invoke).not.toHaveBeenCalledWith('stop_recording')
    act(() => useAppStore.setState({ pipelineState: 'recording', recoveryAlertId: null }))
    expect(screen.queryByText(/Complete sentence/)).not.toBeInTheDocument()
    expect(useAppStore.getState().recoveryEntries).toHaveLength(1)
  })

  it('renders preparing state', () => {
    useAppStore.setState({ pipelineState: 'preparing' })

    render(<Capsule />)

    expect(screen.getByText('capsule.preparing')).toBeInTheDocument()
  })

  it.each(['preparing', 'recording', 'transcribing', 'polishing'] as const)(
    'cancels %s on Escape without submitting recording',
    (pipelineState) => {
      useAppStore.setState({ pipelineState })
      render(<Capsule />)
      fireEvent.keyDown(window, { key: 'Escape' })
      expect(abortRecording).toHaveBeenCalledTimes(1)
      expect(invoke).not.toHaveBeenCalledWith('stop_recording')
    },
  )

  it('switches dictation to live typing with one click and hides the mode chip in live typing', () => {
    useAppStore.setState({ pipelineState: 'recording', liveTyping: false })
    const { unmount } = render(<Capsule />)
    fireEvent.click(screen.getByRole('button', { name: 'capsule.switchToLive' }))
    expect(switchRecordingMode).toHaveBeenCalledWith(true)
    expect(abortRecording).not.toHaveBeenCalled()
    unmount()

    useAppStore.setState({ pipelineState: 'recording', liveTyping: true })
    render(<Capsule />)
    expect(screen.getByText('capsule.liveTyping')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'capsule.switchToStt' }))
    expect(switchRecordingMode).toHaveBeenLastCalledWith(false)
  })

  it('cancels Ask on Escape', () => {
    useAppStore.setState({ pipelineState: 'ask_recording' })
    render(<Capsule />)
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(abortAskDictation).toHaveBeenCalledTimes(1)
    expect(stopAskFlow).not.toHaveBeenCalled()
  })

  it('opens modes without stopping capture and saves a new mode', async () => {
    useAppStore.setState({ pipelineState: 'recording' })
    render(<Capsule />)
    fireEvent.click(screen.getByRole('button', { name: 'settings.polishModeTitle' }))
    expect(screen.getByRole('dialog', { name: 'settings.polishModeTitle' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('radio', { name: 'settings.polishStyleOriginal' }))
    await waitFor(() => expect(setPolishStyle).toHaveBeenCalledWith('original'))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
    expect(useAppStore.getState().config.polish_style).toBe('original')
    expect(invoke).not.toHaveBeenCalledWith('stop_recording')
  })

  it('closes the mode chooser on Escape before cancelling dictation', () => {
    useAppStore.setState({ pipelineState: 'recording', polishModeMenuOpen: true })
    render(<Capsule />)
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(abortRecording).not.toHaveBeenCalled()
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(abortRecording).toHaveBeenCalledTimes(1)
  })

  it('renders transcribing state with partial transcript when available', () => {
    useAppStore.setState({
      pipelineState: 'transcribing',
      partialTranscript: 'hello world',
    })

    render(<Capsule />)

    expect(screen.getByText(/hello world/)).toBeInTheDocument()
  })

  it('renders thinking state during polishing', () => {
    useAppStore.setState({ pipelineState: 'polishing' })

    render(<Capsule />)

    expect(screen.getByText('capsule.thinking')).toBeInTheDocument()
  })

  it('does not start dictation when the idle capsule is clicked', () => {
    const { container } = render(<Capsule />)
    const shell = container.querySelector('.jelly-capsule')
    expect(shell).toBeTruthy()

    const pointerUp = new Event('pointerup', { bubbles: true })
    Object.defineProperty(pointerUp, 'button', { value: 0 })
    fireEvent(shell as Element, pointerUp)

    expect(invoke).not.toHaveBeenCalledWith('start_recording')
  })

  it('renders Ask recording in the capsule and stops Ask when clicked', () => {
    useAppStore.setState({ pipelineState: 'ask_recording' })

    render(<Capsule />)

    expect(screen.getByText('ask.title')).toBeInTheDocument()
    expect(screen.getByText('ask.title')).toHaveClass('whitespace-nowrap')
    expect(screen.getByText('00:00')).toBeInTheDocument()

    const pointerUp = new Event('pointerup', { bubbles: true })
    Object.defineProperty(pointerUp, 'button', { value: 0 })
    fireEvent(screen.getByText('ask.title'), pointerUp)

    expect(stopAskFlow).toHaveBeenCalledTimes(1)
  })

  it('renders Ask thinking in the capsule', () => {
    useAppStore.setState({ pipelineState: 'ask_thinking' })

    render(<Capsule />)

    expect(screen.getByText('ask.title')).toBeInTheDocument()
    expect(screen.getByText('ask.title')).toHaveClass('whitespace-nowrap')
    expect(screen.getByText('ask.thinking')).toBeInTheDocument()
  })

  it('shows the language chip only while Translate is recording', () => {
    useAppStore.setState({
      pipelineState: 'recording',
      activeVoiceMode: 'dictate',
      config: {
        ...useAppStore.getState().config,
        translation: { targets: ['en', 'ja'], active_target: 'en' },
      },
    })
    const { rerender } = render(<Capsule />)

    expect(screen.queryByRole('button', { name: 'capsule.translationTarget en' })).toBeNull()

    useAppStore.setState({ activeVoiceMode: 'translate' })
    rerender(<Capsule />)
    expect(screen.getByRole('button', { name: 'capsule.translationTarget en' })).toBeInTheDocument()

    useAppStore.setState({ pipelineState: 'transcribing' })
    rerender(<Capsule />)
    expect(screen.queryByRole('button', { name: 'capsule.translationTarget en' })).toBeNull()
  })

  it('switches the active target without restarting or stopping recording', async () => {
    useAppStore.setState({
      pipelineState: 'recording',
      activeVoiceMode: 'translate',
      config: {
        ...useAppStore.getState().config,
        translation: { targets: ['en', 'ja'], active_target: 'en' },
      },
    })
    render(<Capsule />)

    fireEvent.pointerDown(screen.getByRole('button', { name: 'capsule.translationTarget en' }))
    fireEvent.click(screen.getByRole('button', { name: 'capsule.translationTarget en' }))
    fireEvent.click(screen.getByRole('menuitemradio', { name: '日本語' }))

    await waitFor(() => expect(setActiveTranslationTarget).toHaveBeenCalledWith('ja'))
    expect(invoke).not.toHaveBeenCalledWith('stop_recording')
    expect(invoke).not.toHaveBeenCalledWith('start_recording')
    expect(useAppStore.getState().config.translation.active_target).toBe('ja')
  })

  it('keeps the recording capsule shell at 228 by 36 and closes the target menu on Escape', async () => {
    useAppStore.setState({
      pipelineState: 'recording',
      activeVoiceMode: 'translate',
      config: {
        ...useAppStore.getState().config,
        translation: { targets: ['en', 'ja'], active_target: 'en' },
      },
    })
    const { container } = render(<Capsule />)
    const chip = screen.getByRole('button', { name: 'capsule.translationTarget en' })

    fireEvent.click(chip)
    expect(screen.getByRole('menu', { name: 'capsule.translationTargets' })).toBeInTheDocument()

    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('menu', { name: 'capsule.translationTargets' })).toBeNull()
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'capsule.translationTarget en' })).toHaveFocus(),
    )

    const shell = container.querySelector('.jelly-capsule-active') as HTMLElement
    expect(shell.style.width).toBe('228px')
    expect(shell.style.height).toBe('36px')
  })
})
