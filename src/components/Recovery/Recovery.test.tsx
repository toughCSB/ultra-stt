import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { Recovery } from './index'
import { useAppStore, type RecoveryEntry } from '../../stores/appStore'
import { copyRecoveryEntry, dismissRecoveryEntry } from '../../lib/tauri'
import i18n from '../../i18n'

vi.mock('../../lib/tauri', () => ({
  copyRecoveryEntry: vi.fn().mockResolvedValue(undefined),
  dismissRecoveryEntry: vi.fn().mockResolvedValue([]),
}))

const entries: RecoveryEntry[] = [
  { id: 'first', sessionId: 1, createdAt: '2026-10-03T00:00:00Z', text: '첫째 완성문\n둘째 줄', appName: 'Editor', reason: 'partial_insert', charsInserted: 2 },
  { id: 'second', sessionId: 2, createdAt: '2026-10-03T00:00:01Z', text: 'Another finished sentence', appName: 'Browser', reason: 'target_changed', charsInserted: 3 },
]

beforeEach(() => {
  void i18n.changeLanguage('ko')
  useAppStore.setState({ ...useAppStore.getInitialState(), recoveryEntries: entries, config: { ...useAppStore.getState().config, history_enabled: false } })
})

afterEach(() => {
  cleanup()
  vi.resetAllMocks()
  vi.mocked(copyRecoveryEntry).mockResolvedValue(undefined)
  vi.mocked(dismissRecoveryEntry).mockResolvedValue([])
  void i18n.changeLanguage('en')
})

describe('Recovery', () => {
  it('shows all completed text even when history saving is off, and copies through native IPC', async () => {
    render(<Recovery />)
    expect(screen.getByText(/첫째 완성문/)).toHaveTextContent('첫째 완성문 둘째 줄')
    expect(screen.getAllByText(/일부 문장이 이미 입력되었을 수 있습니다/)).toHaveLength(2)
    const nativeClipboard = vi.fn()
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: nativeClipboard } })
    fireEvent.click(screen.getAllByRole('button', { name: '전체 문장 복사' })[0])
    await waitFor(() => expect(copyRecoveryEntry).toHaveBeenCalledWith('first'))
    expect(await screen.findByRole('button', { name: '복사됨' })).toBeInTheDocument()
    expect(nativeClipboard).not.toHaveBeenCalled()
  })

  it('keeps the sentence after a copy failure and allows an explicit dismiss of one entry', async () => {
    vi.mocked(copyRecoveryEntry).mockRejectedValueOnce(new Error('clipboard unavailable'))
    render(<Recovery />)
    fireEvent.click(screen.getAllByRole('button', { name: '전체 문장 복사' })[0])
    expect(await screen.findByRole('alert')).toHaveTextContent('복사하지 못했습니다')
    expect(useAppStore.getState().recoveryEntries).toHaveLength(2)

    fireEvent.click(screen.getAllByRole('button', { name: '지우기' })[0])
    await waitFor(() => expect(dismissRecoveryEntry).toHaveBeenCalledWith('first'))
    await waitFor(() => expect(useAppStore.getState().recoveryEntries.map((entry) => entry.id)).toEqual(['second']))
    expect(screen.getByText('Another finished sentence')).toBeInTheDocument()
  })

  it('preserves the entry when dismissal fails', async () => {
    vi.mocked(dismissRecoveryEntry).mockRejectedValueOnce(new Error('native error'))
    render(<Recovery />)
    fireEvent.click(screen.getAllByRole('button', { name: '지우기' })[0])
    expect(await screen.findByRole('alert')).toHaveTextContent('지우지 못했습니다')
    expect(useAppStore.getState().recoveryEntries).toHaveLength(2)
  })
})
