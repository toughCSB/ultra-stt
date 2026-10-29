import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { LlmSetupStep } from '../LlmSetupStep'

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (key: string) => key }) }))
afterEach(() => cleanup())

it('keeps onboarding clear of provider and model selectors or paid tests', () => {
  render(<LlmSetupStep />)
  expect(screen.getByText('settings.engine.polishNote')).toBeInTheDocument()
  expect(screen.queryByRole('combobox')).toBeNull()
  expect(screen.queryByRole('button')).toBeNull()
})
