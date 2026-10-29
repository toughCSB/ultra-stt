import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useAppStore } from '../../../stores/appStore'
import {
  getCredentialStatus,
  readCredential,
  setCredential,
  testEngineSetup,
  type CredentialStatus,
} from '../../../lib/tauri'
import { EnginePane } from '../EnginePane'

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}))

vi.mock('../../../lib/tauri', () => ({
  getCredentialStatus: vi.fn(),
  readCredential: vi.fn(),
  setCredential: vi.fn(),
  testEngineSetup: vi.fn(),
}))

const credentialStatus = (
  hasSecret: boolean,
  storage: CredentialStatus['storage'] = 'os-vault',
): CredentialStatus => ({
  namespace: 'stt',
  provider: 'openai-transcribe',
  hasSecret,
  updatedAt: null,
  storage,
})

beforeEach(() => {
  useAppStore.setState(useAppStore.getInitialState())
  vi.mocked(getCredentialStatus).mockResolvedValue(credentialStatus(false))
  vi.mocked(setCredential).mockResolvedValue(undefined)
  vi.mocked(testEngineSetup).mockResolvedValue({ success: true, message: 'Native ready' })
})

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('EnginePane', () => {
  it('distinguishes the saved engine from a pending selection until Save succeeds', () => {
    const saved = useAppStore.getState().config
    useAppStore.getState().setSavedConfig(saved)
    render(<EnginePane />)

    const existingCard = screen
      .getByRole('radio', { name: 'settings.engine.existingTitle' })
      .closest('label')!
    const openaiCard = screen
      .getByRole('radio', { name: 'settings.engine.openaiTitle' })
      .closest('label')!
    expect(within(existingCard).getByText('settings.engine.savedActive')).toBeInTheDocument()
    expect(within(existingCard).getByText('settings.engine.selected')).toBeInTheDocument()

    fireEvent.click(openaiCard)
    expect(within(openaiCard).getByText('settings.engine.selected')).toBeInTheDocument()
    expect(within(openaiCard).queryByText('settings.engine.savedActive')).toBeNull()
    expect(within(existingCard).getByText('settings.engine.savedActive')).toBeInTheDocument()
    expect(screen.getByText('settings.engine.saveToSwitch')).toBeInTheDocument()
    expect(useAppStore.getState().savedConfig).toBe(saved)
    expect(setCredential).not.toHaveBeenCalled()

    act(() => useAppStore.getState().setSavedConfig(useAppStore.getState().config))
    expect(within(openaiCard).getByText('settings.engine.savedActive')).toBeInTheDocument()
    expect(screen.getByText('settings.engine.nextRecording')).toBeInTheDocument()
  })

  it('does not read or save API credentials while the existing profile is selected', () => {
    render(<EnginePane />)
    expect(getCredentialStatus).not.toHaveBeenCalled()
    expect(setCredential).not.toHaveBeenCalled()
  })

  it('shows two profiles and switches modes without changing legacy connections or styles', async () => {
    useAppStore.getState().updateConfig({
      stt_provider: 'deepgram',
      llm_provider: 'codex-cli',
      llm_model: 'gpt-6-luna',
      polish_style: 'professional',
      stt_language: 'ko',
    })
    const before = useAppStore.getState().config
    render(<EnginePane />)

    expect(screen.getAllByRole('radio')).toHaveLength(2)
    expect(screen.getByRole('radio', { name: 'settings.engine.existingTitle' })).toBeChecked()
    expect(screen.getByText(/gpt-6-luna/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('radio', { name: 'settings.engine.openaiTitle' }))

    const after = useAppStore.getState().config
    expect(after).toEqual({ ...before, engine_mode: 'openai' })
    expect(screen.getByRole('radio', { name: 'settings.engine.openaiTitle' })).toBeChecked()
    await waitFor(() => expect(screen.getByText('settings.engine.keyNotSaved')).toBeInTheDocument())
    expect(
      screen.getByRole('button', { name: 'settings.engine.testSttConnection' }),
    ).toBeInTheDocument()
    expect(testEngineSetup).not.toHaveBeenCalled()
  })

  it('tests the selected STT route only on click and clears success after switching profiles', async () => {
    render(<EnginePane />)
    expect(testEngineSetup).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.testSttConnection' }))
    expect(testEngineSetup).toHaveBeenCalledWith('stt', 'existing', 'gpt-6-luna')
    expect(await screen.findByText('settings.engine.testPassed: Native ready')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('radio', { name: 'settings.engine.openaiTitle' }))
    expect(screen.queryByText('settings.engine.testPassed: Native ready')).toBeNull()
    expect(testEngineSetup).toHaveBeenCalledTimes(1)
  })

  it('keeps a new draft local until explicit Save, then locks the saved key', async () => {
    render(<EnginePane />)
    expect(setCredential).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole('radio', { name: 'settings.engine.openaiTitle' }))
    expect(getCredentialStatus).toHaveBeenCalledWith('stt', 'openai-transcribe')
    const input = await screen.findByLabelText('settings.engine.apiKey')
    fireEvent.change(input, { target: { value: 'manually-entered-key' } })
    fireEvent.blur(input)
    expect(setCredential).not.toHaveBeenCalled()
    expect(screen.queryByText('settings.engine.keySaved')).toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.saveKey' }))

    await waitFor(() =>
      expect(setCredential).toHaveBeenCalledWith(
        'stt',
        'openai-transcribe',
        'manually-entered-key',
      ),
    )
    expect(screen.getByText('settings.engine.keySaved')).toBeInTheDocument()
    expect(screen.getByText('settings.engine.keyStoredLocallyUnverified')).toBeInTheDocument()
    expect(screen.queryByLabelText('settings.engine.apiKey')).toBeNull()
    expect(useAppStore.getState().config.stt_api_key).toBe('')
    expect(useAppStore.getState().config.llm_api_key).toBe('')
  })

  it('loads a saved key into a locked masked summary, never into the edit input', async () => {
    vi.mocked(getCredentialStatus).mockResolvedValue(credentialStatus(true))
    useAppStore.getState().updateConfig({ engine_mode: 'openai' })
    render(<EnginePane />)

    await waitFor(() => expect(screen.getByText('settings.engine.keySaved')).toBeInTheDocument())
    expect(screen.getByLabelText('settings.engine.keyMasked')).toHaveTextContent('••••••••')
    expect(screen.queryByLabelText('settings.engine.apiKey')).toBeNull()
    expect(readCredential).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.modifyKey' }))
    expect(screen.getByLabelText('settings.engine.apiKey')).toHaveValue('')
    expect(setCredential).not.toHaveBeenCalled()
  })

  it('cancels a replacement without touching the stored key', async () => {
    vi.mocked(getCredentialStatus).mockResolvedValue(credentialStatus(true))
    useAppStore.getState().updateConfig({ engine_mode: 'openai' })
    render(<EnginePane />)
    fireEvent.click(await screen.findByRole('button', { name: 'settings.engine.modifyKey' }))
    fireEvent.change(screen.getByLabelText('settings.engine.apiKey'), {
      target: { value: 'replacement-key' },
    })
    fireEvent.blur(screen.getByLabelText('settings.engine.apiKey'))
    expect(setCredential).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.cancelKeyEdit' }))
    expect(screen.getByText('settings.engine.keySaved')).toBeInTheDocument()
    expect(screen.queryByLabelText('settings.engine.apiKey')).toBeNull()
    expect(setCredential).not.toHaveBeenCalled()
  })

  it('keeps the previous saved status and edit draft when replacement fails', async () => {
    vi.mocked(getCredentialStatus).mockResolvedValue(credentialStatus(true))
    vi.mocked(setCredential).mockRejectedValue(new Error('vault unavailable'))
    useAppStore.getState().updateConfig({ engine_mode: 'openai' })
    render(<EnginePane />)
    fireEvent.click(await screen.findByRole('button', { name: 'settings.engine.modifyKey' }))
    fireEvent.change(screen.getByLabelText('settings.engine.apiKey'), {
      target: { value: 'replacement-key' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.saveKey' }))
    await waitFor(() => expect(screen.getByRole('alert')).toBeInTheDocument())
    expect(screen.getByText('settings.engine.previousKeySaved')).toBeInTheDocument()
    expect(screen.queryByText('settings.engine.keySaved')).toBeNull()
    expect(screen.getByLabelText('settings.engine.apiKey')).toHaveValue('replacement-key')
    expect(setCredential).toHaveBeenCalledTimes(1)
  })

  it('does not show a saved key when the first save fails', async () => {
    vi.mocked(setCredential).mockRejectedValue(new Error('vault unavailable'))
    useAppStore.getState().updateConfig({ engine_mode: 'openai' })
    render(<EnginePane />)
    const input = await screen.findByLabelText('settings.engine.apiKey')
    fireEvent.change(input, { target: { value: 'new-key' } })
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.saveKey' }))
    await waitFor(() => expect(screen.getByRole('alert')).toBeInTheDocument())
    expect(screen.queryByText('settings.engine.keySaved')).toBeNull()
    expect(screen.queryByRole('button', { name: 'settings.engine.modifyKey' })).toBeNull()
    expect(input).toHaveValue('new-key')
  })

  it('reloads a successfully replaced key as locked after remount', async () => {
    vi.mocked(getCredentialStatus).mockResolvedValue(credentialStatus(true))
    useAppStore.getState().updateConfig({ engine_mode: 'openai' })
    const { unmount } = render(<EnginePane />)
    fireEvent.click(await screen.findByRole('button', { name: 'settings.engine.modifyKey' }))
    fireEvent.change(screen.getByLabelText('settings.engine.apiKey'), {
      target: { value: 'new-key' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.saveKey' }))
    await waitFor(() => expect(screen.getByText('settings.engine.keySaved')).toBeInTheDocument())
    unmount()
    render(<EnginePane />)
    expect(
      await screen.findByRole('button', { name: 'settings.engine.modifyKey' }),
    ).toBeInTheDocument()
    expect(getCredentialStatus).toHaveBeenCalledTimes(2)
    expect(screen.queryByLabelText('settings.engine.apiKey')).toBeNull()
    expect(setCredential).toHaveBeenCalledTimes(1)
  })

  it('shows a read error rather than an empty-key state, and can retry', async () => {
    vi.mocked(getCredentialStatus)
      .mockRejectedValueOnce(new Error('vault unavailable'))
      .mockResolvedValueOnce(credentialStatus(true))
    useAppStore.getState().updateConfig({ engine_mode: 'openai' })
    render(<EnginePane />)
    expect(screen.getByText('settings.engine.keyLoading')).toBeInTheDocument()
    expect(await screen.findByRole('alert')).toBeInTheDocument()
    expect(screen.queryByText('settings.engine.keyNotSaved')).toBeNull()
    expect(screen.queryByLabelText('settings.engine.apiKey')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'settings.engine.retry' }))
    expect(
      await screen.findByRole('button', { name: 'settings.engine.modifyKey' }),
    ).toBeInTheDocument()
    expect(setCredential).not.toHaveBeenCalled()
  })

  it('treats an unavailable vault status as an error, not a missing key', async () => {
    vi.mocked(getCredentialStatus).mockResolvedValue(credentialStatus(false, 'unavailable'))
    useAppStore.getState().updateConfig({ engine_mode: 'openai' })
    render(<EnginePane />)
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'settings.engine.keyVaultUnavailable',
    )
    expect(screen.queryByText('settings.engine.keyNotSaved')).toBeNull()
    expect(screen.queryByLabelText('settings.engine.apiKey')).toBeNull()
  })

  it('preserves the saved profile, other settings, and key status when switching back and forth', async () => {
    vi.mocked(getCredentialStatus).mockResolvedValue(credentialStatus(true))
    useAppStore.getState().updateConfig({
      engine_mode: 'openai',
      stt_provider: 'deepgram',
      llm_provider: 'codex-cli',
      polish_style: 'professional',
    })
    const saved = useAppStore.getState().config
    useAppStore.getState().setSavedConfig(saved)
    render(<EnginePane />)
    await waitFor(() => expect(screen.getByText('settings.engine.keySaved')).toBeInTheDocument())

    fireEvent.click(
      screen.getByRole('radio', { name: 'settings.engine.existingTitle' }).closest('label')!,
    )
    expect(useAppStore.getState().config).toEqual({ ...saved, engine_mode: 'existing' })
    expect(useAppStore.getState().savedConfig).toBe(saved)
    expect(screen.getByText('settings.engine.saveToSwitch')).toBeInTheDocument()
    expect(screen.getByText(/gpt-6-luna/)).toBeInTheDocument()

    fireEvent.click(
      screen.getByRole('radio', { name: 'settings.engine.openaiTitle' }).closest('label')!,
    )
    expect(useAppStore.getState().config).toEqual(saved)
    await waitFor(() => expect(screen.getByText('settings.engine.keySaved')).toBeInTheDocument())
    expect(screen.queryByLabelText('settings.engine.apiKey')).toBeNull()
    expect(screen.getByText('settings.engine.nextRecording')).toBeInTheDocument()
    expect(setCredential).not.toHaveBeenCalled()
  })
})
