import { useTranslation } from 'react-i18next'

export function LlmSetupStep() {
  const { t } = useTranslation()

  return (
    <div className="rounded-[10px] border border-border bg-bg-secondary p-4 text-[13px] text-text-secondary leading-relaxed">
      {t('settings.engine.polishNote')}
    </div>
  )
}
