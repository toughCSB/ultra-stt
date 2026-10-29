import { describe, expect, it } from 'vitest'
import type { AppConfig } from '../../stores/appStore'
import { useAppStore } from '../../stores/appStore'
import { createBackupSettings, mergeBackupSettings } from '../backup-settings'

describe('createBackupSettings', () => {
  it('round-trips only supported desktop polish models', () => {
    const current = useAppStore.getState().config
    const configured = { ...current, desktop_polish_model: 'claude-sonnet-5-5' as const }
    expect(createBackupSettings(configured).desktop_polish_model).toBe('claude-sonnet-5-5')
    expect(
      mergeBackupSettings(current, { desktop_polish_model: 'grok-4.7' }).desktop_polish_model,
    ).toBe('grok-4.7')
    expect(
      mergeBackupSettings(current, { desktop_polish_model: 'unknown' }).desktop_polish_model,
    ).toBe('gpt-6-luna')
    expect(mergeBackupSettings(current, { desktop_polish_model: 42 }).desktop_polish_model).toBe(
      'gpt-6-luna',
    )
  })

  it('round-trips the engine mode while rejecting unknown backup values', () => {
    const current = useAppStore.getState().config
    const openai = { ...current, engine_mode: 'openai' as const }
    expect(createBackupSettings(openai).engine_mode).toBe('openai')
    expect(mergeBackupSettings(current, { engine_mode: 'openai' }).engine_mode).toBe('openai')
    expect(mergeBackupSettings(current, { engine_mode: 'other' }).engine_mode).toBe('existing')
  })

  it('uses an explicit allow list and keeps only sync-safe family scene assignments', () => {
    const malicious = {
      stt_provider: 'glm-asr',
      stt_api_key: 'stt-secret',
      stt_custom_api_key: 'custom-secret',
      stt_language: 'multi',
      llm_provider: 'openrouter',
      llm_api_key: 'llm-secret',
      llm_model: 'test-model',
      llm_base_url: 'https://example.com/v1',
      context_adaptation_enabled: true,
      custom_scenes: [],
      system_scene_overrides: [{ id: 'system_email', prompt_template: 'Use a warm email body.' }],
      active_scene: null,
      family_scene_assignments: [{ family: 'email', scene_id: 'builtin_professional_email' }],
      custom_app_mappings: [{ matcher: { exactWebHost: 'private.example.com' } }],
      customAppMappings: [{ nativeBundleId: 'com.private.writer' }],
      matcher: { executable: 'private.exe' },
      windowTitle: 'Private document title',
      browserHost: 'private.example.com',
    } as unknown as AppConfig

    const settings = createBackupSettings(malicious)
    const serialized = JSON.stringify(settings)

    expect(settings.family_scene_assignments).toEqual([
      { family: 'email', scene_id: 'builtin_professional_email' },
    ])
    expect(settings.context_adaptation_enabled).toBe(true)
    expect(settings.system_scene_overrides).toEqual([
      { id: 'system_email', prompt_template: 'Use a warm email body.' },
    ])
    for (const forbidden of [
      'stt-secret',
      'custom-secret',
      'llm-secret',
      'custom_app_mappings',
      'customAppMappings',
      'matcher',
      'exactWebHost',
      'nativeBundleId',
      'executable',
      'windowTitle',
      'browserHost',
      'private.example.com',
      'com.private.writer',
      'private.exe',
    ]) {
      expect(serialized).not.toContain(forbidden)
    }
  })

  it('merges only allow-listed settings and preserves local credentials and app matchers', () => {
    const current = {
      ...useAppStore.getState().config,
      llm_api_key: 'local-secret',
      stt_api_key: 'local-stt-secret',
    }

    const merged = mergeBackupSettings(current, {
      polish_enabled: false,
      system_scene_overrides: [{ id: 'system_email', prompt_template: 'Use concise paragraphs.' }],
      llm_api_key: 'cloud-secret',
      stt_api_key: 'cloud-stt-secret',
      custom_app_mappings: [{ matcher: 'private.example.com' }],
    })

    expect(merged.polish_enabled).toBe(false)
    expect(merged.system_scene_overrides).toEqual([
      { id: 'system_email', prompt_template: 'Use concise paragraphs.' },
    ])
    expect(merged.llm_api_key).toBe('local-secret')
    expect(merged.stt_api_key).toBe('local-stt-secret')
    expect(merged).not.toHaveProperty('custom_app_mappings')
  })

  it('round-trips recording-limit intent while retaining the compatibility mirror', () => {
    const configured = {
      ...useAppStore.getState().config,
      recording_limit_mode: 'custom',
      custom_recording_limit_seconds: 300,
      max_recording_seconds: 300,
    } as AppConfig

    const settings = createBackupSettings(configured)
    const restored = mergeBackupSettings(useAppStore.getState().config, settings)

    expect(settings).toMatchObject({
      recording_limit_mode: 'custom',
      custom_recording_limit_seconds: 300,
      max_recording_seconds: 300,
    })
    expect(restored).toMatchObject({
      recording_limit_mode: 'custom',
      custom_recording_limit_seconds: 300,
      max_recording_seconds: 300,
    })
  })
})
