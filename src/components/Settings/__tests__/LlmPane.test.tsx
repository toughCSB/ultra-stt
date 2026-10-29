import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, waitFor, cleanup, within } from '@testing-library/react'
import { LlmPane } from '../LlmPane'
import * as tauri from '../../../lib/tauri'

// Mock Tauri
vi.mock('../../../lib/tauri')

// Mock i18n
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, params?: any) => {
      const translations: Record<string, string> = {
        'settings.provider': 'Provider',
        'nav.upgrade': 'Upgrade',
        'settings.apiKey': 'API Key',
        'settings.model': 'Model',
        'settings.baseUrl': 'Base URL',
        'settings.test': 'Test',
        'settings.enterApiKey': 'Enter API Key',
        'settings.connectionSuccess': 'Connection successful',
        'settings.connectionFailed': 'Connection failed',
        'settings.storedLocally': 'Stored locally',
        'settings.fetchModels': 'Fetch models',
        'settings.modelsAvailable': `${params?.count || 0} models available`,
        'settings.modelCertified': 'Optimized context and thought-aware support',
        'settings.modelBestEffort': 'Context and thought-aware support is best effort',
        'settings.llmModelPlaceholder': 'e.g. gpt-4o-mini',
        'settings.enableAiPolish': 'AI cleanup for dictation',
        'settings.enableAiPolishDesc': 'Cleans up dictation before output',
        'settings.contextAdaptation': 'Adapt writing to the current app',
        'settings.contextAdaptationHint': 'Uses a private local app category',
        'settings.contextAdaptationApps': 'Apps adapted by context',
        'settings.lastDictationContext': 'Last dictation context',
        'settings.browserAccessHint':
          'Allow browser access to use Gmail, Docs, and Slack Web modes.',
        'settings.appStyleMenu': 'App writing style',
        'settings.useDifferentWritingStyle': 'Use a different writing style',
        'settings.manageAppMappings': 'Manage app mappings',
        'settings.appStyleDialogTitle': 'Writing style for this app',
        'settings.polishStyle': 'Polish style',
        'settings.polishStyleOriginal': 'Original',
        'settings.polishStyleOriginalHint':
          'Preserve wording, order, and tone without creating lists or headings.',
        'settings.polishStyleMinimal': 'Minimal',
        'settings.polishStyleClean': 'Clean',
        'settings.polishStyleStructured': 'Structured',
        'settings.polishStyleProfessional': 'Professional',
        'settings.advancedPolishSettings': 'Advanced polish settings',
        'settings.advancedPolishSettingsDesc': 'Optional writing rules',
        'settings.customPolishInstructions': 'Custom polish instructions',
        'settings.customPolishInstructionsPlaceholder': 'Example prompt',
        'settings.customPolishInstructionsCount': `${params?.count || 0} / 2000 characters`,
        'settings.activeScene': `Active scene: ${params?.name || ''}`,
        'settings.clearActiveScene': 'Clear scene',
        'settings.translationMode': 'Always translate output',
        'settings.translationModeDesc': 'Translate each dictation result',
        'settings.selectedTextContext': 'Use selected text as context',
        'settings.selectedTextContextDesc': 'Use selected text for context',
        'settings.targetLanguage': 'Translate to',
        'settings.manageTranslationTargets': 'Manage languages',
        'settings.cloudLlmPro': 'Cloud LLM (Pro)',
        'settings.llmSignInHint':
          'Sign in and subscribe to Pro to use cloud AI polish. No API key needed.',
        'settings.llmUpgradeHint':
          'Upgrade to Pro for cloud AI polish and monthly usage. No API key needed.',
        'settings.llmProActive': 'Pro active — cloud AI polish is ready. No API key needed.',
        'settings.askAnything': 'Ask Anything',
        'settings.askAnythingDesc': 'Voice question, one-shot answer. No chat history.',
        'ask.ready': 'Ready to ask',
        'ask.listening': 'Listening',
        'ask.thinking': 'Thinking',
        'ask.voiceQuestion': 'Voice question',
        'ask.voiceQuestionDesc': 'Speak your question. Stop recording to answer.',
        'ask.transcriptLabel': 'Question transcript',
        'ask.answerLabel': 'Answer',
        'ask.manualFallback': 'Type instead',
        'ask.placeholder': 'Type a question, or use the capsule above.',
        'ask.recordQuestion': 'Record question',
        'ask.stopAndAsk': 'Stop and ask',
        'ask.send': 'Ask',
        'providers.llm.doubao': 'Doubao (Volcengine)',
      }
      return translations[key] || key
    },
  }),
}))

// Mock stores - must be done before importing the component
const mockAppStore = {
  config: {
    engine_mode: 'existing' as 'existing' | 'openai',
    desktop_polish_model: 'gpt-6-luna',
    llm_provider: 'openai' as string,
    llm_api_key: '',
    llm_base_url: 'https://api.openai.com/v1',
    llm_model: 'gpt-4o-mini',
    polish_enabled: true,
    context_adaptation_enabled: true,
    polish_style: 'clean',
    polish_custom_prompt: '',
    polish_chinese_script: 'preserve',
    custom_scenes: [],
    active_scene: null as any,
    family_scene_assignments: [],
    translate_enabled: false,
    selected_text_enabled: false,
    target_lang: 'en',
    translation: { targets: ['en'], active_target: 'en' },
  },
  updateConfig: vi.fn(),
  setConfig: vi.fn(),
  setSavedConfig: vi.fn(),
  llmTestStatus: 'idle' as 'idle' | 'testing' | 'success' | 'error',
  setLlmTestStatus: vi.fn(),
  llmLatencyMs: null as number | null,
  setLlmLatencyMs: vi.fn(),
  llmModels: [] as string[],
  setLlmModels: vi.fn(),
  lastContext: null as any,
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

describe('LlmPane', () => {
  beforeEach(() => {
    // Reset mock store state
    mockAppStore.config = {
      engine_mode: 'existing',
      desktop_polish_model: 'gpt-6-luna',
      llm_provider: 'openai',
      llm_api_key: '',
      llm_base_url: 'https://api.openai.com/v1',
      llm_model: 'gpt-4o-mini',
      polish_enabled: true,
      context_adaptation_enabled: true,
      polish_style: 'clean',
      polish_custom_prompt: '',
      polish_chinese_script: 'preserve',
      custom_scenes: [],
      active_scene: null,
      family_scene_assignments: [],
      translate_enabled: false,
      selected_text_enabled: false,
      target_lang: 'en',
      translation: { targets: ['en'], active_target: 'en' },
    }
    mockAppStore.llmTestStatus = 'idle'
    mockAppStore.llmLatencyMs = null
    mockAppStore.llmModels = []
    mockAppStore.lastContext = null
    mockAuthStore.user = null
    mockAuthStore.plan = null
    mockAuthStore.source = 'free'
    mockAuthStore.cloudWordsLimit = 0
    mockAuthStore.licenseStatus = null

    vi.clearAllMocks()
    vi.mocked(tauri.readCredential).mockResolvedValue(null)
    vi.mocked(tauri.setCredential).mockResolvedValue(undefined)
    vi.mocked(tauri.getLlmModelCapability).mockResolvedValue('unknown')
    vi.mocked(tauri.getLatestMappingCandidate).mockResolvedValue(null)
    vi.mocked(tauri.listCustomAppMappings).mockResolvedValue([])
    vi.mocked(tauri.testEngineSetup).mockResolvedValue({ success: true, message: 'Ready' })
  })

  afterEach(() => {
    cleanup()
    vi.clearAllMocks()
  })

  describe('AI polish behavior settings', () => {
    it('offers exactly six desktop models while preserving five polish styles', () => {
      render(<LlmPane />)
      const modelSelect = screen.getByRole('combobox', {
        name: 'settings.engine.desktopPolishModel',
      })
      expect(
        within(modelSelect)
          .getAllByRole('option')
          .map((option) => option.getAttribute('value')),
      ).toEqual([
        'gpt-6.1-sol',
        'gpt-6.1-terra',
        'gpt-6-luna',
        'claude-haiku-5-5',
        'claude-sonnet-5-5',
        'grok-4.7',
      ])
      expect(within(screen.getByDisplayValue('Clean')).getAllByRole('option')).toHaveLength(5)
      for (const option of within(modelSelect).getAllByRole('option').slice(0, 5)) {
        expect(option).toBeEnabled()
      }
      const grokOption = within(modelSelect).getByRole('option', {
        name: 'settings.engine.grokPendingOption',
      })
      expect(grokOption).toBeDisabled()
      fireEvent.change(modelSelect, { target: { value: 'grok-4.7' } })
      expect(mockAppStore.updateConfig).not.toHaveBeenCalled()
      fireEvent.change(modelSelect, { target: { value: 'claude-sonnet-5-5' } })
      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({
        desktop_polish_model: 'claude-sonnet-5-5',
      })
    })

    it('explains API, Codex CLI, and local Claude/Grok routes without an API key input', () => {
      const { rerender } = render(<LlmPane />)
      expect(screen.getByText('settings.engine.codexCliRoute')).toBeInTheDocument()
      mockAppStore.config.engine_mode = 'openai'
      rerender(<LlmPane />)
      expect(screen.getByText('settings.engine.openAiApiRoute')).toBeInTheDocument()
      mockAppStore.config.desktop_polish_model = 'claude-haiku-5-5'
      rerender(<LlmPane />)
      expect(screen.getByText('settings.engine.localCliRoute')).toBeInTheDocument()
      mockAppStore.config.desktop_polish_model = 'grok-4.7'
      rerender(<LlmPane />)
      expect(screen.getByText('settings.engine.grokPendingNote')).toBeInTheDocument()
      expect(
        screen.getByRole('button', { name: 'settings.engine.testLlmConnection' }),
      ).toBeDisabled()
      expect(tauri.testEngineSetup).not.toHaveBeenCalled()
      expect(screen.getByText('settings.engine.desktopRoutingNote')).toBeInTheDocument()
      expect(screen.queryByLabelText('API Key')).toBeNull()
    })

    it('shows connection failure reason and clears it after model changes', async () => {
      vi.mocked(tauri.testEngineSetup).mockResolvedValue({
        success: false,
        message: 'CLI login required',
      })
      const { rerender } = render(<LlmPane />)
      expect(tauri.testEngineSetup).not.toHaveBeenCalled()
      fireEvent.click(screen.getByRole('button', { name: 'settings.engine.testLlmConnection' }))
      expect(tauri.testEngineSetup).toHaveBeenCalledWith('llm', 'existing', 'gpt-6-luna')
      expect(await screen.findByRole('alert')).toHaveTextContent('CLI login required')
      mockAppStore.config.desktop_polish_model = 'claude-sonnet-5-5'
      rerender(<LlmPane />)
      expect(screen.queryByRole('alert')).toBeNull()
    })

    it('offers Original and shows its preservation guidance when selected', () => {
      render(<LlmPane />)
      fireEvent.change(screen.getByDisplayValue('Clean'), { target: { value: 'original' } })
      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({ polish_style: 'original' })
      mockAppStore.config.polish_style = 'original'
      cleanup()
      render(<LlmPane />)
      expect(screen.getByDisplayValue('Original')).toBeInTheDocument()
      expect(
        screen.getByText('Preserve wording, order, and tone without creating lists or headings.'),
      ).toBeInTheDocument()
    })

    it('shows Clean as the default polish style outside advanced settings', () => {
      render(<LlmPane />)

      expect(screen.getByText('Polish style')).toBeInTheDocument()
      expect(screen.getByDisplayValue('Clean')).toBeInTheDocument()
    })

    it('updates the selected polish style without opening advanced settings', () => {
      render(<LlmPane />)

      fireEvent.change(screen.getByDisplayValue('Clean'), { target: { value: 'structured' } })

      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({ polish_style: 'structured' })
      expect(screen.queryByText('Custom polish instructions')).not.toBeInTheDocument()
    })

    it('keeps custom instruction controls inside advanced settings', () => {
      render(<LlmPane />)

      expect(screen.getByText('Advanced polish settings')).toBeInTheDocument()
      expect(screen.queryByText('Optional writing rules')).not.toBeInTheDocument()
      expect(screen.queryByText('Chinese output')).not.toBeInTheDocument()
      expect(screen.queryByText('Custom polish instructions')).not.toBeInTheDocument()
      expect(screen.queryByText('Use selected text as context')).not.toBeInTheDocument()

      fireEvent.click(screen.getByRole('button', { name: /advanced polish settings/i }))

      expect(screen.getByText('Custom polish instructions')).toBeInTheDocument()
      expect(screen.getByText('Use selected text as context')).toBeInTheDocument()
      expect(screen.getByText('Use selected text for context')).toBeInTheDocument()
      expect(screen.queryByText('Chinese output')).not.toBeInTheDocument()
    })

    it('keeps selected-text controls reachable when cleanup is disabled', () => {
      mockAppStore.config.polish_enabled = false
      render(<LlmPane />)

      fireEvent.click(screen.getByRole('button', { name: /advanced polish settings/i }))

      expect(screen.getByText('Use selected text as context')).toBeInTheDocument()
      expect(screen.queryByText('Custom polish instructions')).not.toBeInTheDocument()
    })

    it('updates custom polish instructions from advanced settings', () => {
      render(<LlmPane />)

      fireEvent.click(screen.getByRole('button', { name: /advanced polish settings/i }))
      const textarea = screen.getByPlaceholderText('Example prompt')
      fireEvent.change(textarea, { target: { value: 'Keep a concise professional tone.' } })

      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({
        polish_custom_prompt: 'Keep a concise professional tone.',
      })
    })

    it('opens advanced settings automatically when custom instructions exist', () => {
      mockAppStore.config.polish_custom_prompt = 'Keep it concise.'

      render(<LlmPane />)

      expect(screen.getByText('Custom polish instructions')).toBeInTheDocument()
      expect(screen.queryByText('Chinese output')).not.toBeInTheDocument()
    })

    it('does not expose legacy global active scenes in AI Polish', () => {
      mockAppStore.config.active_scene = {
        id: 'custom_meeting',
        source: 'custom',
        name: 'Meeting Notes',
        prompt_template: 'Use bullets.',
      }

      render(<LlmPane />)

      expect(screen.queryByText('Active scene: Meeting Notes')).not.toBeInTheDocument()
      expect(screen.queryByText('Clear scene')).not.toBeInTheDocument()
    })
  })

  describe('Feature toggles', () => {
    it('keeps context adaptation adjacent to AI polish and disables it when polish is off', () => {
      mockAppStore.config.polish_enabled = false
      render(<LlmPane />)

      const contextSwitch = screen
        .getByText('Adapt writing to the current app')
        .closest('label')
        ?.querySelector('[role="switch"]')
      expect(contextSwitch).toBeDisabled()
    })

    it('updates the context adaptation preference', () => {
      render(<LlmPane />)
      const contextSwitch = screen
        .getByText('Adapt writing to the current app')
        .closest('label')
        ?.querySelector('[role="switch"]')
      expect(contextSwitch).not.toBeNull()
      fireEvent.click(contextSwitch!)
      expect(mockAppStore.updateConfig).toHaveBeenCalledWith({
        context_adaptation_enabled: false,
      })
    })

    it('shows a compact noninteractive line of representative adapted apps', () => {
      render(<LlmPane />)

      const coverage = screen.getByLabelText('Apps adapted by context')
      for (const name of [
        'Gmail',
        'Slack',
        'Lark',
        'WeChat',
        'Google Docs',
        'Notion',
        'GitHub',
        'Cursor',
      ]) {
        expect(within(coverage).getByLabelText(name)).toBeInTheDocument()
      }
      expect(within(coverage).getByText('+63')).toBeInTheDocument()
      expect(within(coverage).getByText('+65')).toHaveClass('context-app-count--compact')
      expect(within(coverage).getByLabelText('Lark')).toHaveClass('context-app--compact-duplicate')
      expect(within(coverage).getByLabelText('Notion')).toHaveClass(
        'context-app--compact-duplicate',
      )
      expect(within(coverage).queryByRole('button')).not.toBeInTheDocument()
      expect(within(coverage).queryByRole('link')).not.toBeInTheDocument()
      expect(coverage).toHaveAttribute('aria-disabled', 'false')
      expect(coverage).toHaveClass('gap-1')
    })

    it('dims representative apps whenever app adaptation is not active', () => {
      mockAppStore.config.context_adaptation_enabled = false
      render(<LlmPane />)

      expect(screen.getByLabelText('Apps adapted by context')).toHaveAttribute(
        'aria-disabled',
        'true',
      )
    })

    it('keeps helper copy and advanced toggles out of the default flow', () => {
      render(<LlmPane />)

      expect(screen.queryByText('Cleans up dictation before output')).not.toBeInTheDocument()
      expect(screen.queryByText('Translate each dictation result')).not.toBeInTheDocument()
      expect(screen.queryByText('Uses a private local app category')).not.toBeInTheDocument()
      expect(screen.queryByText('Use selected text as context')).not.toBeInTheDocument()
      expect(screen.queryByText('Use selected text for context')).not.toBeInTheDocument()
    })

    it('hides last context until an operation snapshot exists', () => {
      render(<LlmPane />)
      expect(screen.queryByText('Last dictation context')).not.toBeInTheDocument()
    })

    it('shows only the safe last operation context after dictation', () => {
      mockAppStore.lastContext = {
        profileId: 'chat.slack',
        family: 'work_chat',
        appLabel: 'Slack',
        iconKey: 'slack',
        overrideId: 'slack',
        browserAccessStatus: 'available',
      }
      render(<LlmPane />)

      expect(screen.getByText('Last dictation context')).toBeInTheDocument()
      expect(screen.getByText('Slack')).toBeInTheDocument()
      expect(screen.queryByText(/window|host|confidence/i)).not.toBeInTheDocument()
    })

    it('shows a short browser access hint when browser context needs URL access', () => {
      mockAppStore.lastContext = {
        profileId: 'general.browser',
        family: 'general',
        appLabel: 'Browser',
        iconKey: 'general',
        overrideId: null,
        browserAccessStatus: 'needs_permission',
      }

      render(<LlmPane />)

      expect(
        screen.getByText('Allow browser access to use Gmail, Docs, and Slack Web modes.'),
      ).toBeInTheDocument()
    })

    it('keeps the app-style overflow hidden without a live candidate or user mappings', async () => {
      mockAppStore.lastContext = {
        profileId: 'chat.slack',
        family: 'work_chat',
        appLabel: 'Slack',
        iconKey: 'slack',
        overrideId: 'slack',
        browserAccessStatus: 'available',
      }

      render(<LlmPane />)

      await waitFor(() => expect(tauri.listCustomAppMappings).toHaveBeenCalled())
      expect(screen.queryByRole('button', { name: 'App writing style' })).not.toBeInTheDocument()
    })

    it('opens one compact writing-style dialog from a live safe candidate', async () => {
      mockAppStore.lastContext = {
        profileId: 'general.browser',
        family: 'general',
        appLabel: 'Example',
        iconKey: 'general',
        overrideId: null,
        browserAccessStatus: 'available',
      }
      vi.mocked(tauri.getLatestMappingCandidate).mockResolvedValue({
        generation: 7,
        matcherType: 'exact_web_host',
        displayValue: 'docs.example.com',
        suggestedLabel: 'docs.example.com',
        currentFamily: 'document',
        iconKey: 'general',
      })

      render(<LlmPane />)

      fireEvent.click(await screen.findByRole('button', { name: 'App writing style' }))
      fireEvent.click(screen.getByText('Use a different writing style'))

      expect(
        await screen.findByRole('dialog', { name: 'Writing style for this app' }),
      ).toBeVisible()
      expect(screen.getByText('docs.example.com')).toBeInTheDocument()
    })

    it('shows mapping management only for user-created mappings', async () => {
      mockAppStore.lastContext = {
        profileId: 'chat.slack',
        family: 'work_chat',
        appLabel: 'Slack',
        iconKey: 'slack',
        overrideId: 'slack',
        browserAccessStatus: 'available',
      }
      vi.mocked(tauri.listCustomAppMappings).mockResolvedValue([
        {
          id: 'mapping-1',
          label: 'Work Slack',
          matcherType: 'native_bundle_id',
          displayValue: 'Work Slack · macOS',
          family: 'work_chat',
          sceneId: null,
          enabled: true,
          iconKey: 'slack',
        },
      ])

      render(<LlmPane />)

      fireEvent.click(await screen.findByRole('button', { name: 'App writing style' }))
      expect(screen.getByText('Manage app mappings')).toBeInTheDocument()
      expect(screen.queryByText('Use a different writing style')).not.toBeInTheDocument()
      expect(screen.queryByText('Gmail')).not.toBeInTheDocument()
    })

    it('shows a compact translate-to control next to translation when enabled', () => {
      mockAppStore.config.translate_enabled = true

      render(<LlmPane />)
      expect(screen.getByText('Translate to')).toBeInTheDocument()
      expect(screen.queryByRole('radio')).not.toBeInTheDocument()
    })
  })
})
