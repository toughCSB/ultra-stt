import type { PolishStyle } from '../stores/appStore'

export const POLISH_MODES: Array<{
  value: PolishStyle
  labelKey: string
  descriptionKey: string
  position: string
}> = [
  {
    value: 'original',
    labelKey: 'settings.polishStyleOriginal',
    descriptionKey: 'settings.polishModeOriginalDescription',
    position: '0% 0%',
  },
  {
    value: 'minimal',
    labelKey: 'settings.polishStyleMinimal',
    descriptionKey: 'settings.polishModeMinimalDescription',
    position: '50% 0%',
  },
  {
    value: 'clean',
    labelKey: 'settings.polishStyleClean',
    descriptionKey: 'settings.polishModeCleanDescription',
    position: '100% 0%',
  },
  {
    value: 'structured',
    labelKey: 'settings.polishStyleStructured',
    descriptionKey: 'settings.polishModeStructuredDescription',
    position: '0% 100%',
  },
  {
    value: 'professional',
    labelKey: 'settings.polishStyleProfessional',
    descriptionKey: 'settings.polishModeProfessionalDescription',
    position: '50% 100%',
  },
]

export function polishModeLabelKey(style: PolishStyle) {
  return POLISH_MODES.find((mode) => mode.value === style)?.labelKey ?? 'settings.polishStyleClean'
}
