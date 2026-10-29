import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import {
  addCorrectionRule,
  clearCredential,
  commitDictionaryImport,
  exportDictionaryCsv,
  exportDictionaryJson,
  getCorrectionRules,
  loadOnboardingCompleted,
  migrateLegacyCredentials,
  previewDictionaryImport,
  removeCorrectionRule,
  setCorrectionRuleEnabled,
  saveOnboardingCompleted,
  testEngineSetup,
  updateCorrectionRule,
  updateDictionaryEntry,
  waitForAccessibilityPermission,
} from '../tauri'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('profile onboarding commands', () => {
  beforeEach(() => vi.clearAllMocks())

  it('reads and saves completion through the native profile store', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(true).mockResolvedValueOnce(undefined)

    await expect(loadOnboardingCompleted()).resolves.toBe(true)
    await expect(saveOnboardingCompleted()).resolves.toBeUndefined()

    expect(invoke).toHaveBeenNthCalledWith(1, 'load_onboarding_completed')
    expect(invoke).toHaveBeenNthCalledWith(2, 'save_onboarding_completed')
  })
})

describe('waitForAccessibilityPermission', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('returns immediately when accessibility is already trusted', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(true)

    await expect(waitForAccessibilityPermission()).resolves.toBe(true)

    expect(invoke).toHaveBeenCalledTimes(1)
    expect(invoke).toHaveBeenCalledWith('check_accessibility_permission')
  })

  it('polls until accessibility becomes trusted', async () => {
    vi.useFakeTimers()
    vi.mocked(invoke).mockResolvedValueOnce(false).mockResolvedValueOnce(true)

    const result = waitForAccessibilityPermission({ timeoutMs: 1_000, intervalMs: 10 })
    await vi.advanceTimersByTimeAsync(10)

    await expect(result).resolves.toBe(true)
    expect(invoke).toHaveBeenCalledTimes(2)
  })

  it('returns false after the timeout expires', async () => {
    vi.useFakeTimers()
    vi.mocked(invoke).mockResolvedValue(false)

    const result = waitForAccessibilityPermission({ timeoutMs: 20, intervalMs: 10 })
    await vi.advanceTimersByTimeAsync(20)

    await expect(result).resolves.toBe(false)
    expect(invoke).toHaveBeenCalledTimes(3)
  })
})

describe('credential commands', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('clears credentials through the explicit backend command', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)

    await clearCredential('llm', 'openai')

    expect(invoke).toHaveBeenCalledWith('clear_credential', {
      namespace: 'llm',
      provider: 'openai',
    })
  })

  it('runs the explicit legacy credential migration command', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)

    await migrateLegacyCredentials()

    expect(invoke).toHaveBeenCalledWith('migrate_legacy_credentials')
  })
})

describe('engine setup test command', () => {
  it('passes the selected desktop route without user audio or text', async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ success: false, message: 'Sign in locally' })

    await expect(testEngineSetup('llm', 'existing', 'claude-haiku-5-5')).resolves.toEqual({
      success: false,
      message: 'Sign in locally',
    })
    expect(invoke).toHaveBeenCalledWith('test_engine_setup', {
      kind: 'llm',
      engineMode: 'existing',
      desktopPolishModel: 'claude-haiku-5-5',
    })
  })
})

describe('dictionary correction commands', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('reads correction rules from the backend command', async () => {
    vi.mocked(invoke).mockResolvedValueOnce([
      { id: 1, pattern: '拓肯', replacement: 'Token', enabled: true },
    ])

    await expect(getCorrectionRules()).resolves.toEqual([
      { id: 1, pattern: '拓肯', replacement: 'Token', enabled: true },
    ])

    expect(invoke).toHaveBeenCalledWith('get_correction_rules')
  })

  it('adds a correction rule through the backend command', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)

    await addCorrectionRule('拓肯', 'Token')

    expect(invoke).toHaveBeenCalledWith('add_correction_rule', {
      pattern: '拓肯',
      replacement: 'Token',
    })
  })

  it('removes a correction rule through the backend command', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)

    await removeCorrectionRule(7)

    expect(invoke).toHaveBeenCalledWith('remove_correction_rule', { id: 7 })
  })

  it('toggles a correction rule through the backend command', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined)

    await setCorrectionRuleEnabled(7, false)

    expect(invoke).toHaveBeenCalledWith('set_correction_rule_enabled', {
      id: 7,
      enabled: false,
    })
  })

  it('updates dictionary and correction rows through typed commands', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined)

    await updateDictionaryEntry(4, 'OpenTypeless', 'open typeless')
    await updateCorrectionRule(7, 'open type less', 'OpenTypeless', false)

    expect(invoke).toHaveBeenNthCalledWith(1, 'update_dictionary_entry', {
      id: 4,
      word: 'OpenTypeless',
      pronunciation: 'open typeless',
    })
    expect(invoke).toHaveBeenNthCalledWith(2, 'update_correction_rule', {
      id: 7,
      pattern: 'open type less',
      replacement: 'OpenTypeless',
      enabled: false,
    })
  })

  it('previews and commits identical import bytes and exports content only', async () => {
    const report = {
      accepted: 1,
      skippedDuplicates: 0,
      skippedInvalid: 0,
      errors: [],
    }
    vi.mocked(invoke)
      .mockResolvedValueOnce(report)
      .mockResolvedValueOnce(report)
      .mockResolvedValueOnce('{}')
      .mockResolvedValueOnce('type,word')

    await expect(previewDictionaryImport([65], 'txt')).resolves.toEqual(report)
    await expect(commitDictionaryImport([65], 'txt')).resolves.toEqual(report)
    await expect(exportDictionaryJson()).resolves.toBe('{}')
    await expect(exportDictionaryCsv()).resolves.toBe('type,word')

    expect(invoke).toHaveBeenNthCalledWith(1, 'preview_dictionary_import', {
      bytes: [65],
      format: 'txt',
    })
    expect(invoke).toHaveBeenNthCalledWith(2, 'commit_dictionary_import', {
      bytes: [65],
      format: 'txt',
    })
    expect(invoke).toHaveBeenNthCalledWith(3, 'export_dictionary_json')
    expect(invoke).toHaveBeenNthCalledWith(4, 'export_dictionary_csv')
  })
})
