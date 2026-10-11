import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { RecordingRecovery } from './RecordingRecovery'
import * as api from '../../lib/tauri'
import i18n from '../../i18n'

vi.mock('../../lib/tauri', () => ({
  listRecordingArchives: vi.fn(),
  retryRecordingArchive: vi.fn(),
  deleteRecordingArchive: vi.fn(),
  copyRecordingTranscript: vi.fn(),
  openRecordingArchiveFolder: vi.fn(),
}))

const recording: api.ArchivedRecording = {
  id: 'recording-id',
  createdAt: '2026-10-11T00:00:00Z',
  durationSeconds: 600,
  transcript: '',
  active: false,
}

beforeEach(() => {
  vi.resetAllMocks()
  void i18n.changeLanguage('en')
  vi.mocked(api.listRecordingArchives).mockResolvedValue([{ ...recording }])
  vi.mocked(api.openRecordingArchiveFolder).mockResolvedValue(undefined)
  vi.mocked(api.copyRecordingTranscript).mockResolvedValue(undefined)
  vi.mocked(api.deleteRecordingArchive).mockResolvedValue(undefined)
})

afterEach(cleanup)

describe('RecordingRecovery', () => {
  it('retries transcription and shows the saved original without automatic insertion', async () => {
    vi.mocked(api.retryRecordingArchive).mockImplementation(async () => {
      vi.mocked(api.listRecordingArchives).mockResolvedValue([
        { ...recording, transcript: 'Recovered original text' },
      ])
      return 'Recovered original text'
    })
    render(<RecordingRecovery />)
    expect(await screen.findByText('10:00')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: i18n.t('audioRecovery.retry') }))
    expect(await screen.findByText('Recovered original text')).toBeInTheDocument()
    expect(api.retryRecordingArchive).toHaveBeenCalledWith(recording.id)
    expect(api.deleteRecordingArchive).not.toHaveBeenCalled()
    expect(api.copyRecordingTranscript).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: i18n.t('recovery.copy') }))
    await waitFor(() => expect(api.copyRecordingTranscript).toHaveBeenCalledWith(recording.id))
  })

  it('keeps the recording visible after a retry failure', async () => {
    vi.mocked(api.retryRecordingArchive).mockRejectedValue(new Error('server offline'))
    render(<RecordingRecovery />)
    await screen.findByText('10:00')
    fireEvent.click(screen.getByRole('button', { name: i18n.t('audioRecovery.retry') }))
    expect(await screen.findByRole('alert')).toHaveTextContent('server offline')
    expect(screen.getByText('10:00')).toBeInTheDocument()
    expect(api.deleteRecordingArchive).not.toHaveBeenCalled()
  })

  it('requires a second click to delete and never allows deletion or retry of active audio', async () => {
    vi.mocked(api.listRecordingArchives).mockResolvedValue([
      recording,
      { ...recording, id: 'active-recording', durationSeconds: 300, active: true },
    ])
    render(<RecordingRecovery />)
    await screen.findByText('5:00')
    const removeButtons = screen.getAllByRole('button', { name: i18n.t('audioRecovery.delete') })
    expect(removeButtons[1]).toBeDisabled()
    expect(screen.getAllByRole('button', { name: i18n.t('audioRecovery.retry') })[1]).toBeDisabled()
    fireEvent.click(removeButtons[0])
    expect(api.deleteRecordingArchive).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: i18n.t('audioRecovery.confirmDelete') }))
    await waitFor(() => expect(api.deleteRecordingArchive).toHaveBeenCalledWith(recording.id))
  })

  it('opens only the app-managed recovery folder through native IPC', async () => {
    render(<RecordingRecovery />)
    await screen.findByText('10:00')
    fireEvent.click(screen.getByRole('button', { name: i18n.t('audioRecovery.folder') }))
    await waitFor(() => expect(api.openRecordingArchiveFolder).toHaveBeenCalledOnce())
  })
})
