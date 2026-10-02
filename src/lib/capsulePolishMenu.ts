import { useAppStore } from '../stores/appStore'

export function togglePolishModeMenu() {
  const state = useAppStore.getState()
  state.setContextMenuOpen(false)
  state.setTranslationTargetMenuOpen(false)
  state.setPolishModeMenuOpen(!state.polishModeMenuOpen)
}
