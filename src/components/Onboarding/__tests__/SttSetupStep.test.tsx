import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { useAppStore } from '../../../stores/appStore'
import { SttSetupStep } from '../SttSetupStep'

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (key: string) => key }) }))
vi.mock('../../../lib/tauri', () => ({
  getCredentialStatus: vi.fn().mockResolvedValue({ hasSecret: false, storage: 'os-vault' }),
  setCredential: vi.fn().mockResolvedValue(undefined),
}))

beforeEach(() => useAppStore.setState(useAppStore.getInitialState()))
afterEach(() => cleanup())

it('offers only the two engine profiles without switching legacy providers', () => {
  const before = useAppStore.getState().config
  render(<SttSetupStep />)
  expect(screen.getAllByRole('radio')).toHaveLength(2)
  expect(screen.queryByRole('combobox')).toBeNull()
  fireEvent.click(screen.getByRole('radio', { name: 'settings.engine.openaiTitle' }))
  expect(useAppStore.getState().config).toEqual({ ...before, engine_mode: 'openai' })
})
