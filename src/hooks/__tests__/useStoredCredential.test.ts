import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { readCredential, setCredential } from '../../lib/tauri'
import { useStoredCredential } from '../useStoredCredential'

vi.mock('../../lib/tauri', () => ({
  readCredential: vi.fn(),
  setCredential: vi.fn(),
}))

afterEach(() => {
  vi.clearAllMocks()
  vi.useRealTimers()
})

describe('useStoredCredential', () => {
  it('loads the vault key, but a key left in the settings file wins', async () => {
    vi.mocked(readCredential).mockResolvedValue('vault-key')

    const fromVault = renderHook(() =>
      useStoredCredential({ namespace: 'stt', provider: 'glm-asr', enabled: true, legacyKey: '' }),
    )
    await waitFor(() => expect(fromVault.result.current.draft).toBe('vault-key'))
    expect(readCredential).toHaveBeenCalledWith('stt', 'glm-asr')

    const legacy = renderHook(() =>
      useStoredCredential({
        namespace: 'llm',
        provider: 'openai',
        enabled: true,
        legacyKey: 'old',
      }),
    )
    await waitFor(() => expect(readCredential).toHaveBeenCalledTimes(2))
    expect(legacy.result.current.draft).toBe('old')
  })

  it('saves once typing pauses and reports a failed save', async () => {
    vi.mocked(readCredential).mockResolvedValue(null)
    vi.mocked(setCredential).mockRejectedValueOnce('vault locked')
    vi.useFakeTimers()
    const { result } = renderHook(() =>
      useStoredCredential({ namespace: 'llm', provider: 'openai', enabled: true, legacyKey: '' }),
    )

    act(() => {
      result.current.persist('s')
      result.current.persist('sk')
    })
    await act(async () => {
      await vi.advanceTimersByTimeAsync(349)
    })
    expect(setCredential).not.toHaveBeenCalled()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1)
    })
    expect(setCredential).toHaveBeenCalledTimes(1)
    expect(setCredential).toHaveBeenCalledWith('llm', 'openai', 'sk')
    expect(result.current.saveError).toBe('vault locked')
  })

  it('keeps nothing for a provider that needs no key', () => {
    const { result } = renderHook(() =>
      useStoredCredential({
        namespace: 'stt',
        provider: 'apple-speech',
        enabled: false,
        legacyKey: 'stale',
      }),
    )
    act(() => result.current.persist('ignored', 0))

    expect(result.current.draft).toBe('')
    expect(readCredential).not.toHaveBeenCalled()
    expect(setCredential).not.toHaveBeenCalled()
  })

  it('does not overwrite a typed key when an earlier vault read finishes late', async () => {
    let finishRead: (value: string | null) => void = () => {}
    vi.mocked(readCredential).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finishRead = resolve
        }),
    )
    const { result } = renderHook(() =>
      useStoredCredential({
        namespace: 'stt',
        provider: 'openai-transcribe',
        enabled: true,
        legacyKey: '',
      }),
    )
    act(() => {
      result.current.setDraft('manual-key')
      result.current.persist('manual-key')
    })
    await act(async () => finishRead('old-vault-key'))
    expect(result.current.draft).toBe('manual-key')
    expect(result.current.stored).toBe(false)
  })
})
