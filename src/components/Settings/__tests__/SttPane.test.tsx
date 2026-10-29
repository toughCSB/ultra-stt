import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, cleanup, within } from '@testing-library/react'
import { SttPane } from '../SttPane'
import * as tauri from '../../../lib/tauri'

// Mock Tauri
vi.mock('../../../lib/tauri')

// Mock i18n
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, values?: Record<string, string | number>) => {
      const translations: Record<string, string> = {
        'settings.provider': 'Provider',
        'nav.upgrade': 'Upgrade',
        'settings.apiKey': 'API Key',
        'settings.test': 'Test',
        'settings.enterApiKey': 'Enter API Key',
        'settings.connectionSuccess': 'Connection successful',
        'settings.connectionFailed': 'Connection failed',
        'settings.storedLocally': 'Stored locally',
        'settings.sttLanguage': 'STT Language',
        'settings.maxRecordingDuration': 'Single recording duration',
        'recordingLimits.auto': 'Auto (recommended, up to {{duration}})',
        'recordingLimits.custom': 'Custom',
        'recordingLimits.customDuration': 'Custom duration',
        'recordingLimits.allowedRange': 'Supported range: {{min}}–{{max}}.',
        'recordingLimits.allowedRangeWithReason': 'Range: {{min}}–{{max}}. {{reason}}',
        'recordingLimits.allowedCloudRange':
          'Range: {{min}}–{{max}}. Set by current Cloud capability.',
        'recordingLimits.corrected': 'This provider will use {{duration}}.',
        'recordingLimits.currentSelectionWithCloudMax':
          'Current: {{current}}. Cloud supports up to {{max}}.',
        'recordingLimits.currentSelectionWithLimit':
          'Selected: {{current}}; app limit: {{max}}. {{reason}}',
        'recordingLimits.providerFixedLimit': 'This provider allows recordings up to {{max}}.',
        'recordingLimits.durationSeconds': '{{count}} seconds',
        'recordingLimits.durationMinute': '{{count}} minute',
        'recordingLimits.durationMinutes': '{{count}} minutes',
        'recordingLimits.secondsUnit': 'seconds',
        'recordingLimits.presets': 'Recording limit presets',
        'recordingLimits.numericEntry': 'Custom duration…',
        'recordingLimits.reasons.productSafety': 'Product safety limit',
        'recordingLimits.reasons.providerDuration': 'Provider duration limit',
        'recordingLimits.reasons.appleSpeech': 'Apple Speech session limit',
        'recordingLimits.reasons.clientBuffer': 'Client buffer limit',
        'recordingLimits.reasons.unknownUpstream': 'Upstream limit unknown',
        'recordingLimits.reasons.unknownProvider': 'Provider limit unknown',
        'recordingLimits.reasons.managedCapability': 'Set automatically for OpenTypeless Cloud.',
        'recordingLimits.reasons.managedFallback': 'Safe Cloud fallback',
        'settings.cloudSttPro': 'Cloud STT (Pro)',
        'settings.sttSignInHint': 'Sign in to use cloud STT',
        'settings.sttUpgradeHint': 'Upgrade to Pro to use cloud STT',
        'settings.sttProActive': 'Cloud STT active',
        'settings.customSttPreset': 'Preset',
        'settings.customSttPresetSpeaches': 'Speaches',
        'settings.customSttPresetCustom': 'Custom OpenAI-compatible',
        'settings.customSttBaseUrl': 'Base URL',
        'settings.customSttBaseUrlPlaceholder': 'http://localhost:8000/v1',
        'settings.customSttModel': 'Model',
        'settings.customSttModelPlaceholder': 'Systran/faster-whisper-large-v3',
        'settings.customSttApiKeyOptional': 'API Key (optional)',
        'settings.customSttSetupHint':
          'Start your local OpenAI-compatible STT server first, then test the connection here.',
        'settings.localSttReady': 'Local endpoint ready',
        'settings.localSttNeedsSetup': 'Local endpoint needs setup',
        'settings.appleSpeechReady': 'Apple Speech ready',
        'settings.appleSpeechUnavailable': 'Apple Speech unavailable',
        'settings.customSttConnectionFailed':
          'Local STT server is not reachable. Check that it is running and the port is correct.',
        'settings.volcengineSttKeyHint':
          'Use a Volcengine Speech API key, or app_id:access_token from the old console. Ark LLM keys are separate.',
        'settings.volcengineResourceId': 'Volcengine ASR resource',
        'settings.volcengineResourceSeedAsr': 'SeedASR 2.0',
        'settings.volcengineResourceBigAsr': 'BigASR 1.0',
        'settings.aliyunQwenRegion': 'Service region',
        'settings.aliyunQwenRegionChina': 'China Mainland (Beijing)',
        'settings.aliyunQwenRegionInternational': 'International (Singapore)',
        'settings.aliyunQwenRegionHint':
          'Use the region where your DashScope API key was created. Keys are region-specific.',
        'providers.stt.aliyunQwen3Asr': 'Aliyun Qwen3 Realtime ASR',
        'providers.stt.volcengineDoubao': 'Volcengine Doubao Realtime ASR',
        'providers.stt.appleSpeech': 'Apple Speech (Local)',
      }
      return Object.entries(values ?? {}).reduce(
        (text, [name, value]) => text.replace(`{{${name}}}`, String(value)),
        translations[key] || key,
      )
    },
  }),
}))

// Mock stores
const mockAppStore = {
  config: {
    engine_mode: 'existing' as 'existing' | 'openai',
    stt_provider: 'deepgram' as string,
    stt_api_key: '',
    stt_custom_api_key: '',
    stt_language: 'en',
    stt_custom_preset: 'speaches',
    stt_custom_base_url: 'http://localhost:8000/v1',
    stt_custom_model: 'Systran/faster-whisper-large-v3',
    stt_volcengine_resource_id: 'volc.seedasr.sauc.duration',
    stt_aliyun_qwen_region: 'china-mainland' as 'china-mainland' | 'international',
    recording_limit_mode: 'auto' as 'auto' | 'custom',
    custom_recording_limit_seconds: 600,
    max_recording_seconds: 600,
  },
  updateConfig: vi.fn(),
  sttTestStatus: 'idle' as 'idle' | 'testing' | 'success' | 'error',
  setSttTestStatus: vi.fn(),
  sttLatencyMs: null as number | null,
  setSttLatencyMs: vi.fn(),
  platformCapabilities: {
    os: 'macos',
    sessionType: 'unknown',
    globalHotkeyReliable: true,
    keyboardOutputReliable: true,
    clipboardAutoPasteReliable: true,
  },
}

const mockAuthStore = {
  user: null as any,
  plan: null as any,
  source: 'free',
  cloudWordsLimit: 0,
  licenseStatus: null as any,
}

vi.mock('../../../stores/appStore', () => ({
  useAppStore: (selector: any) => {
    if (typeof selector === 'function') {
      return selector(mockAppStore)
    }
    return mockAppStore
  },
}))

vi.mock('../../../stores/authStore', () => ({
  hasManagedCloudAccess: (state: typeof mockAuthStore) =>
    state.licenseStatus !== 'refunded' &&
    state.licenseStatus !== 'deactivated' &&
    ((state.source === 'creem' && state.cloudWordsLimit > 0) ||
      (state.source === 'appsumo' &&
        state.cloudWordsLimit > 0 &&
        state.licenseStatus === 'active') ||
      state.plan === 'pro'),
  useAuthStore: (selector: any) => {
    if (typeof selector === 'function') {
      return selector(mockAuthStore)
    }
    return mockAuthStore
  },
}))

describe('SttPane', () => {
  beforeEach(() => {
    // Reset mock store state
    mockAppStore.config = {
      engine_mode: 'existing',
      stt_provider: 'deepgram',
      stt_api_key: '',
      stt_custom_api_key: '',
      stt_language: 'en',
      stt_custom_preset: 'speaches',
      stt_custom_base_url: 'http://localhost:8000/v1',
      stt_custom_model: 'Systran/faster-whisper-large-v3',
      stt_volcengine_resource_id: 'volc.seedasr.sauc.duration',
      stt_aliyun_qwen_region: 'china-mainland',
      recording_limit_mode: 'auto',
      custom_recording_limit_seconds: 600,
      max_recording_seconds: 600,
    }
    mockAppStore.sttTestStatus = 'idle'
    mockAppStore.sttLatencyMs = null
    mockAppStore.platformCapabilities = {
      os: 'macos',
      sessionType: 'unknown',
      globalHotkeyReliable: true,
      keyboardOutputReliable: true,
      clipboardAutoPasteReliable: true,
    }
    mockAuthStore.user = null
    mockAuthStore.plan = null
    mockAuthStore.source = 'free'
    mockAuthStore.cloudWordsLimit = 0
    mockAuthStore.licenseStatus = null

    // Clear all mock function calls
    vi.clearAllMocks()
    vi.mocked(tauri.readCredential).mockResolvedValue(null)
    vi.mocked(tauri.setCredential).mockResolvedValue(undefined)
    vi.mocked(tauri.getSttRecordingCapability).mockResolvedValue({
      capability: {
        registryVersion: 1,
        providerId: 'deepgram',
        transport: 'streaming',
        recommendedMaxSeconds: 600,
        hardMaxSeconds: 3600,
        maxUploadBytes: null,
        source: 'productSafety',
        explanationKey: 'recordingLimits.reasons.productSafety',
      },
      mode: 'auto',
      requestedSeconds: 600,
      effectiveMaxSeconds: 600,
    })
    vi.mocked(tauri.getSttProviderDiagnostics).mockResolvedValue({
      provider: 'custom-whisper',
      kind: 'localCompatible',
      endpoint: 'http://localhost:8000/v1/audio/transcriptions',
      model: 'Systran/faster-whisper-large-v3',
      requiresApiKey: false,
      apiKeyConfigured: false,
      ready: true,
      issues: [],
    })
  })

  afterEach(() => {
    cleanup()
    vi.clearAllMocks()
  })

  describe('Recording limit', () => {
    it('resolves the OpenAI engine recording limit without changing the stored provider', async () => {
      mockAppStore.config.engine_mode = 'openai'
      render(<SttPane />)
      expect(tauri.getSttRecordingCapability).toHaveBeenCalledWith('openai-transcribe', 'auto', 600)
      expect(mockAppStore.config.stt_provider).toBe('deepgram')
    })

    it('shows the Rust-resolved Auto value and governing reason', async () => {
      render(<SttPane />)

      expect(
        await screen.findByRole('option', { name: /Auto \(recommended,.*10 minutes/i }),
      ).toBeInTheDocument()
      expect(screen.getByText('Product safety limit')).toBeInTheDocument()
      expect(tauri.getSttRecordingCapability).toHaveBeenCalledWith('deepgram', 'auto', 600)
    })

    it('filters choices in the single duration selector using the Rust hard maximum', async () => {
      mockAppStore.config.stt_provider = 'glm-asr'
      mockAppStore.config.recording_limit_mode = 'custom'
      mockAppStore.config.custom_recording_limit_seconds = 600
      vi.mocked(tauri.getSttRecordingCapability).mockResolvedValueOnce({
        capability: {
          registryVersion: 1,
          providerId: 'glm-asr',
          transport: 'fileUpload',
          recommendedMaxSeconds: 30,
          hardMaxSeconds: 30,
          maxUploadBytes: 24 * 1024 * 1024,
          source: 'provider',
          explanationKey: 'recordingLimits.reasons.providerDuration',
        },
        mode: 'custom',
        requestedSeconds: 600,
        effectiveMaxSeconds: 30,
      })

      render(<SttPane />)

      const duration = await screen.findByLabelText('Single recording duration')
      expect(within(duration).getByRole('option', { name: '30 seconds' })).toBeInTheDocument()
      expect(within(duration).queryByRole('option', { name: '1 minute' })).not.toBeInTheDocument()
      expect(
        within(duration).queryByRole('option', { name: 'Custom duration…' }),
      ).not.toBeInTheDocument()
      expect(
        screen.getByText('This provider allows recordings up to 30 seconds.'),
      ).toBeInTheDocument()
      expect(screen.getByText('This provider will use 30 seconds.')).toBeInTheDocument()
    })

    it('shows one selected duration and distinguishes it from the Cloud maximum', async () => {
      mockAppStore.config.stt_provider = 'cloud'
      mockAppStore.config.recording_limit_mode = 'custom'
      mockAppStore.config.custom_recording_limit_seconds = 30
      vi.mocked(tauri.getSttRecordingCapability).mockResolvedValueOnce({
        capability: {
          registryVersion: 1,
          providerId: 'cloud',
          transport: 'managedUpload',
          recommendedMaxSeconds: 600,
          hardMaxSeconds: 600,
          maxUploadBytes: 4_000_000,
          source: 'managedProduct',
          explanationKey: 'recordingLimits.reasons.managedCapability',
        },
        mode: 'custom',
        requestedSeconds: 30,
        effectiveMaxSeconds: 30,
      })

      render(<SttPane />)

      const duration = await screen.findByLabelText('Single recording duration')
      expect(duration).toHaveValue('30')
      expect(screen.queryByLabelText('Recording limit presets')).not.toBeInTheDocument()
      expect(screen.queryByLabelText('Custom duration')).not.toBeInTheDocument()
      expect(
        screen.getByText('Current: 30 seconds. Cloud supports up to 10 minutes.'),
      ).toBeInTheDocument()
    })

    it('reveals a bounded numeric entry only after Custom duration is selected', async () => {
      mockAppStore.config.stt_provider = 'cloud'
      vi.mocked(tauri.getSttRecordingCapability).mockResolvedValueOnce({
        capability: {
          registryVersion: 1,
          providerId: 'cloud',
          transport: 'managedUpload',
          recommendedMaxSeconds: 600,
          hardMaxSeconds: 600,
          maxUploadBytes: 4_000_000,
          source: 'managedProduct',
          explanationKey: 'recordingLimits.reasons.managedCapability',
        },
        mode: 'auto',
        requestedSeconds: 600,
        effectiveMaxSeconds: 600,
      })
      render(<SttPane />)

      const duration = await screen.findByLabelText('Single recording duration')
      expect(screen.queryByLabelText('Custom duration')).not.toBeInTheDocument()

      fireEvent.change(duration, { target: { value: 'custom' } })
      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({ recording_limit_mode: 'custom' })

      const input = await screen.findByLabelText('Custom duration')
      expect(input).toHaveAttribute('min', '30')
      expect(input).toHaveAttribute('max', '600')
      expect(screen.getByText('seconds')).toBeInTheDocument()
      expect(
        screen.getByText('Range: 30 seconds–10 minutes. Set by current Cloud capability.'),
      ).toBeInTheDocument()
      expect(
        screen.queryByText('Current: 10 minutes. Cloud supports up to 10 minutes.'),
      ).not.toBeInTheDocument()
      fireEvent.change(input, { target: { value: '300' } })
      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({
        custom_recording_limit_seconds: 300,
      })
    })

    it('describes an app buffer limit without claiming it is provider support', async () => {
      mockAppStore.config.stt_provider = 'openai-whisper'
      mockAppStore.config.recording_limit_mode = 'custom'
      mockAppStore.config.custom_recording_limit_seconds = 600
      vi.mocked(tauri.getSttRecordingCapability).mockResolvedValueOnce({
        capability: {
          registryVersion: 1,
          providerId: 'openai-whisper',
          transport: 'fileUpload',
          recommendedMaxSeconds: 600,
          hardMaxSeconds: 720,
          maxUploadBytes: 24 * 1024 * 1024,
          source: 'clientBuffer',
          explanationKey: 'recordingLimits.reasons.clientBuffer',
        },
        mode: 'custom',
        requestedSeconds: 600,
        effectiveMaxSeconds: 600,
      })

      render(<SttPane />)

      expect(
        await screen.findByText('Selected: 10 minutes; app limit: 12 minutes. Client buffer limit'),
      ).toBeInTheDocument()
      expect(
        screen.queryByText('Current: 10 minutes. This service supports up to 12 minutes.'),
      ).not.toBeInTheDocument()
    })

    it('keeps the managed fallback explanation and hides meaningless custom entry', async () => {
      mockAppStore.config.stt_provider = 'cloud'
      mockAppStore.config.recording_limit_mode = 'custom'
      mockAppStore.config.custom_recording_limit_seconds = 30
      vi.mocked(tauri.getSttRecordingCapability).mockResolvedValueOnce({
        capability: {
          registryVersion: 1,
          providerId: 'cloud',
          transport: 'managedUpload',
          recommendedMaxSeconds: 30,
          hardMaxSeconds: 30,
          maxUploadBytes: 4_000_000,
          source: 'managedProduct',
          explanationKey: 'recordingLimits.reasons.managedFallback',
        },
        mode: 'custom',
        requestedSeconds: 30,
        effectiveMaxSeconds: 30,
      })

      render(<SttPane />)

      const duration = await screen.findByLabelText('Single recording duration')
      expect(
        within(duration).queryByRole('option', { name: 'Custom duration…' }),
      ).not.toBeInTheDocument()
      expect(screen.getByText('Safe Cloud fallback')).toBeInTheDocument()
      expect(
        screen.queryByText('Current: 30 seconds. Cloud supports up to 30 seconds.'),
      ).not.toBeInTheDocument()
    })

    it('stores a preset as a custom duration from the single selector', async () => {
      render(<SttPane />)
      const duration = await screen.findByLabelText('Single recording duration')

      fireEvent.change(duration, { target: { value: '300' } })

      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({
        recording_limit_mode: 'custom',
        custom_recording_limit_seconds: 300,
      })
    })

    it('shows 30 seconds for stale Cloud metadata and 10 minutes only for compatible v2', async () => {
      mockAppStore.config.stt_provider = 'cloud'
      vi.mocked(tauri.getSttRecordingCapability).mockResolvedValueOnce({
        capability: {
          registryVersion: 1,
          providerId: 'cloud',
          transport: 'managedUpload',
          recommendedMaxSeconds: 30,
          hardMaxSeconds: 30,
          maxUploadBytes: 4_000_000,
          source: 'managedProduct',
          explanationKey: 'recordingLimits.reasons.managedFallback',
        },
        mode: 'auto',
        requestedSeconds: 30,
        effectiveMaxSeconds: 30,
      })
      render(<SttPane />)

      expect(
        await screen.findByRole('option', { name: /Auto \(recommended,.*30 seconds/i }),
      ).toBeInTheDocument()
      expect(screen.getByText('Safe Cloud fallback')).toBeInTheDocument()

      cleanup()
      vi.mocked(tauri.getSttRecordingCapability).mockResolvedValueOnce({
        capability: {
          registryVersion: 1,
          providerId: 'cloud',
          transport: 'managedUpload',
          recommendedMaxSeconds: 600,
          hardMaxSeconds: 600,
          maxUploadBytes: 4_000_000,
          source: 'managedProduct',
          explanationKey: 'recordingLimits.reasons.managedCapability',
        },
        mode: 'auto',
        requestedSeconds: 600,
        effectiveMaxSeconds: 600,
      })
      render(<SttPane />)

      expect(
        await screen.findByRole('option', { name: /Auto \(recommended,.*10 minutes/i }),
      ).toBeInTheDocument()
      expect(screen.getByText('Set automatically for OpenTypeless Cloud.')).toBeInTheDocument()
    })
  })

  describe('Language selection', () => {
    it('renders language dropdown with current value', () => {
      render(<SttPane />)
      const selects = screen.getAllByRole('combobox')
      const languageSelect = selects[0]
      expect(languageSelect).toHaveValue('en')
    })

    it('updates config when language changes', () => {
      render(<SttPane />)
      const selects = screen.getAllByRole('combobox')
      const languageSelect = selects[0]

      fireEvent.change(languageSelect, { target: { value: 'zh' } })

      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({ stt_language: 'zh' })
    })
  })
})
