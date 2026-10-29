import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { listen } from '@tauri-apps/api/event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { testEngineSetup } from '../../../lib/tauri'
import { EngineConnectionTest } from '../EngineConnectionTest'

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}))

vi.mock('../../../lib/tauri', () => ({
  testEngineSetup: vi.fn(),
}))

beforeEach(() => {
  vi.mocked(testEngineSetup).mockResolvedValue({ success: true, message: 'Ready' })
})

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('EngineConnectionTest', () => {
  it('runs only on click and shows the backend reason on failure', async () => {
    vi.mocked(testEngineSetup).mockResolvedValue({
      success: false,
      message: 'Local CLI sign-in required',
    })
    render(
      <EngineConnectionTest
        kind="llm"
        engineMode="existing"
        desktopPolishModel="claude-sonnet-5-5"
      />,
    )

    expect(testEngineSetup).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.testLlmConnection' }))
    expect(testEngineSetup).toHaveBeenCalledWith('llm', 'existing', 'claude-sonnet-5-5')
    expect(await screen.findByRole('alert')).toHaveTextContent('Local CLI sign-in required')
  })

  it('clears a successful test when credentials change and ignores an older response', async () => {
    let resolveTest: (result: { success: boolean; message: string }) => void = () => {}
    vi.mocked(testEngineSetup).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveTest = resolve
        }),
    )
    render(<EngineConnectionTest kind="stt" engineMode="openai" desktopPolishModel="gpt-6-luna" />)
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.testSttConnection' }))
    expect(screen.getByRole('button', { name: 'settings.engine.testingConnection' })).toBeDisabled()

    await waitFor(() =>
      expect(listen).toHaveBeenCalledWith('credentials:changed', expect.any(Function)),
    )
    act(() => vi.mocked(listen).mock.calls[0][1]({} as never))
    expect(screen.getByRole('button', { name: 'settings.engine.testSttConnection' })).toBeEnabled()
    await act(async () => resolveTest({ success: false, message: 'Stale result' }))
    expect(screen.queryByRole('alert')).toBeNull()
    expect(screen.queryByRole('status')).toBeNull()
  })

  it('shows a thrown IPC reason and clears it when the route changes', async () => {
    vi.mocked(testEngineSetup).mockRejectedValue(new Error('CLI executable missing'))
    const { rerender } = render(
      <EngineConnectionTest
        key="existing:gpt"
        kind="llm"
        engineMode="existing"
        desktopPolishModel="gpt-6-luna"
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.testLlmConnection' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('CLI executable missing')

    rerender(
      <EngineConnectionTest
        key="openai:gpt"
        kind="llm"
        engineMode="openai"
        desktopPolishModel="gpt-6-luna"
      />,
    )
    expect(screen.queryByRole('alert')).toBeNull()
    expect(testEngineSetup).toHaveBeenCalledTimes(1)
  })
})
