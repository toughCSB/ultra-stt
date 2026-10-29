export const DESKTOP_POLISH_MODELS = [
  'gpt-6.1-sol',
  'gpt-6.1-terra',
  'gpt-6-luna',
  'claude-haiku-5-5',
  'claude-sonnet-5-5',
  'grok-4.7',
] as const

export type DesktopPolishModel = (typeof DESKTOP_POLISH_MODELS)[number]

export function isDesktopPolishModel(value: unknown): value is DesktopPolishModel {
  return DESKTOP_POLISH_MODELS.some((model) => model === value)
}
