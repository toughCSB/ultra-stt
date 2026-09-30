import { act, cleanup, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import App from './App'
import i18n from './i18n'
import { getConfig, loadOnboardingCompleted } from './lib/tauri'
import { useAppStore, type AppConfig } from './stores/appStore'

const auth = vi.hoisted(() => ({ initialize: vi.fn() }))

vi.mock('./hooks/useTauriEvents', () => ({ useTauriEvents: vi.fn() }))
vi.mock('./hooks/useTheme', () => ({ useTheme: vi.fn() }))
vi.mock('./lib/router', () => ({ useRoute: () => ({ route: 'home', navigate: vi.fn() }) }))
vi.mock('./lib/deep-link', () => ({ initDeepLinkListener: vi.fn() }))
vi.mock('./i18n', () => ({ default: { language: 'en', changeLanguage: vi.fn() } }))
vi.mock('./lib/tauri', () => ({
  loadOnboardingCompleted: vi.fn(),
  getConfig: vi.fn(),
}))
vi.mock('./stores/authStore', () => {
  const state = { user: null, loading: false }
  const useAuthStore = Object.assign(
    (selector: (value: typeof state) => unknown) => selector(state),
    { getState: () => auth },
  )
  return { useAuthStore }
})
vi.mock('./components/Onboarding', () => ({
  Onboarding: () => <div>Onboarding loaded</div>,
}))
vi.mock('./components/Capsule', () => ({ Capsule: () => null }))
vi.mock('./components/Settings', () => ({ Settings: () => null }))
vi.mock('./components/History', () => ({ History: () => null }))
vi.mock('./components/MainLayout', () => ({ MainLayout: () => null }))
vi.mock('./components/HomePage', () => ({ HomePage: () => null }))
vi.mock('./components/UpgradePage', () => ({ UpgradePage: () => null }))
vi.mock('./components/AccountPage', () => ({ AccountPage: () => null }))
vi.mock('./components/AskPanel', () => ({ AskPanel: () => null }))
vi.mock('./components/Toast', () => ({ ToastContainer: () => null }))

beforeEach(() => {
  useAppStore.setState(useAppStore.getInitialState())
  window.location.hash = ''
  vi.clearAllMocks()
  vi.mocked(loadOnboardingCompleted).mockResolvedValue(false)
})

afterEach(cleanup)

it('loads native personal-profile defaults before rendering unsaved onboarding', async () => {
  const nativeConfig: AppConfig = {
    ...useAppStore.getState().config,
    stt_provider: 'custom-whisper',
    stt_custom_preset: 'ultra-local',
    stt_custom_base_url: 'http://127.0.0.1:17845/v1',
    llm_provider: 'codex-cli',
    llm_model: 'gpt-6-luna',
    ui_language: 'ko',
  }
  let releaseConfig!: (config: AppConfig) => void
  vi.mocked(getConfig).mockImplementation(
    () => new Promise<AppConfig>((resolve) => { releaseConfig = resolve }),
  )

  await act(async () => {
    render(<App />)
  })
  expect(getConfig).toHaveBeenCalledOnce()
  expect(screen.queryByText('Onboarding loaded')).toBeNull()

  await act(async () => {
    releaseConfig(nativeConfig)
  })
  expect(screen.getByText('Onboarding loaded')).toBeInTheDocument()
  expect(useAppStore.getState().config).toMatchObject({
    stt_provider: 'custom-whisper',
    stt_custom_preset: 'ultra-local',
    llm_provider: 'codex-cli',
    llm_model: 'gpt-6-luna',
  })
  expect(useAppStore.getState().savedConfig).toEqual(nativeConfig)
  expect(i18n.changeLanguage).toHaveBeenCalledWith('ko')
})
