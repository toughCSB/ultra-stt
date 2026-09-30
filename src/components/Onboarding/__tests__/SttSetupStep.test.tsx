import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { SttSetupStep } from '../SttSetupStep'

const mockStore = {
  config: {
    stt_provider: 'deepgram',
    stt_api_key: '',
    stt_custom_api_key: '',
    stt_custom_preset: 'custom',
    stt_custom_base_url: 'http://localhost:8000/v1',
    stt_custom_model: 'Systran/faster-whisper-large-v3',
  },
  updateConfig: vi.fn(),
  sttTestStatus: 'idle',
  setSttTestStatus: vi.fn(),
}

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        'onboarding.stt.serviceLabel': 'Service',
        'onboarding.stt.apiKeyLabel': 'API key',
        'onboarding.stt.apiKeyPlaceholder': 'API key',
        'onboarding.stt.testButton': 'Test',
        'onboarding.stt.connectionOk': 'OK',
        'onboarding.stt.connectionFail': 'Failed',
        'onboarding.stt.customWhisperConfigured': 'Custom Whisper',
        'providers.stt.deepgram': 'Deepgram',
        'providers.stt.customWhisper': 'Custom Whisper',
      })[key] ?? key,
  }),
}))

vi.mock('../../../stores/appStore', () => ({
  useAppStore: (selector: any) => selector(mockStore),
}))

vi.mock('../../../lib/tauri', () => ({
  testSttConnection: vi.fn().mockResolvedValue(true),
}))

beforeEach(() => {
  mockStore.config = {
    stt_provider: 'deepgram',
    stt_api_key: '',
    stt_custom_api_key: '',
    stt_custom_preset: 'custom',
    stt_custom_base_url: 'http://localhost:8000/v1',
    stt_custom_model: 'Systran/faster-whisper-large-v3',
  }
  mockStore.updateConfig = vi.fn()
  mockStore.sttTestStatus = 'idle'
  mockStore.setSttTestStatus = vi.fn()
})

afterEach(() => cleanup())

describe('SttSetupStep', () => {
  it('does not offer managed Cloud inside BYOK provider setup', () => {
    render(<SttSetupStep />)

    const providerSelect = screen.getByRole('combobox')
    expect(providerSelect.querySelector('option[value="cloud"]')).toBeNull()
  })

  it('preserves an existing Custom Whisper setup instead of switching providers', () => {
    mockStore.config = {
      ...mockStore.config,
      stt_provider: 'custom-whisper',
      stt_custom_base_url: 'http://localhost:9000/v1',
      stt_custom_model: 'local-large-v3',
    }

    render(<SttSetupStep />)

    expect(screen.getByRole('combobox')).toHaveValue('custom-whisper')
    expect(screen.getByText('http://localhost:9000/v1')).toBeInTheDocument()
    expect(screen.getByText('local-large-v3')).toBeInTheDocument()
    expect(mockStore.updateConfig).not.toHaveBeenCalledWith({ stt_provider: 'deepgram' })
  })

  it('tests Custom Whisper with its configured endpoint and model', async () => {
    const tauri = await import('../../../lib/tauri')
    mockStore.config = {
      ...mockStore.config,
      stt_provider: 'custom-whisper',
      stt_custom_api_key: 'custom-secret',
      stt_custom_base_url: 'http://localhost:9000/v1',
      stt_custom_model: 'local-large-v3',
    }

    render(<SttSetupStep />)
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Test' }))
    })
    expect(tauri.testSttConnection).toHaveBeenCalledWith(
      'custom-secret',
      'custom-whisper',
      'http://localhost:9000/v1',
      'local-large-v3',
    )
  })

  it('selects the managed local endpoint without requiring an API key', () => {
    render(<SttSetupStep />)
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'ultra-local' } })
    expect(mockStore.updateConfig).toHaveBeenCalledWith({
      stt_provider: 'custom-whisper',
      stt_custom_preset: 'ultra-local',
      stt_custom_base_url: 'http://127.0.0.1:17845/v1',
      stt_custom_model: 'Systran/faster-whisper-large-v3',
      stt_custom_api_key: '',
    })
  })

  it('tests the managed route without forwarding a stale provider key', async () => {
    const tauri = await import('../../../lib/tauri')
    mockStore.config = {
      ...mockStore.config,
      stt_provider: 'custom-whisper',
      stt_custom_preset: 'ultra-local',
      stt_custom_api_key: 'stale-key',
      stt_custom_base_url: 'http://127.0.0.1:17845/v1',
      stt_custom_model: 'Systran/faster-whisper-large-v3',
    }
    const { container } = render(<SttSetupStep />)
    expect(container.querySelector('input[type="password"]')).toBeNull()
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Test' }))
    })
    expect(tauri.testSttConnection).toHaveBeenLastCalledWith(
      '',
      'custom-whisper',
      'http://127.0.0.1:17845/v1',
      'Systran/faster-whisper-large-v3',
    )
    expect(mockStore.setSttTestStatus).toHaveBeenLastCalledWith('success')
  })
})
