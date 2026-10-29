import { useTranslation } from 'react-i18next'
import { Check, Cloud, Workflow } from 'lucide-react'
import { useAppStore } from '../../stores/appStore'
import { EngineConnectionTest } from './EngineConnectionTest'
import { OpenAiCredentialPanel } from './OpenAiCredentialPanel'

export function EnginePane() {
  const { t } = useTranslation()
  const config = useAppStore((state) => state.config)
  const savedConfig = useAppStore((state) => state.savedConfig)
  const updateConfig = useAppStore((state) => state.updateConfig)
  const selectedMode = config.engine_mode === 'openai' ? 'openai' : 'existing'
  const savedMode = savedConfig?.engine_mode
  const pendingSwitch = savedMode !== undefined && selectedMode !== savedMode

  return (
    <div className="space-y-5">
      <p className="text-[12px] text-text-secondary">{t('settings.engine.chooseHint')}</p>
      <div role="radiogroup" aria-label={t('settings.engineSelection')} className="grid gap-3">
        {(['existing', 'openai'] as const).map((mode) => {
          const isSelected = selectedMode === mode
          const isSaved = savedMode === mode
          const Icon = mode === 'existing' ? Workflow : Cloud
          const color = mode === 'existing' ? 'green' : 'blue'

          return (
            <label
              key={mode}
              className={`relative block min-h-32 rounded-xl border-2 p-5 cursor-pointer transition-colors focus-within:ring-2 focus-within:ring-offset-2 focus-within:ring-offset-bg-primary ${
                color === 'green'
                  ? isSelected
                    ? 'border-green-500 bg-green-500/10 focus-within:ring-green-500'
                    : 'border-border bg-bg-secondary hover:border-green-500/60 focus-within:ring-green-500'
                  : isSelected
                    ? 'border-blue-500 bg-blue-500/10 focus-within:ring-blue-500'
                    : 'border-border bg-bg-secondary hover:border-blue-500/60 focus-within:ring-blue-500'
              }`}
            >
              <input
                type="radio"
                name="engine-mode"
                value={mode}
                aria-label={t(`settings.engine.${mode}Title`)}
                className="sr-only"
                checked={isSelected}
                onChange={() => updateConfig({ engine_mode: mode })}
              />
              <span className="flex items-start gap-3">
                <span
                  className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-lg ${
                    mode === 'existing'
                      ? 'bg-green-500/15 text-green-600'
                      : 'bg-blue-500/15 text-blue-600'
                  }`}
                >
                  <Icon size={22} aria-hidden="true" />
                </span>
                <span className="min-w-0 flex-1">
                  <span className="flex flex-wrap items-center gap-2">
                    <span className="text-[15px] font-semibold text-text-primary">
                      {t(`settings.engine.${mode}Title`)}
                    </span>
                    {isSelected && (
                      <span className="rounded-full bg-bg-primary px-2 py-0.5 text-[11px] font-medium text-text-primary">
                        {t('settings.engine.selected')}
                      </span>
                    )}
                    {isSaved && (
                      <span className="inline-flex items-center gap-1 rounded-full bg-bg-primary px-2 py-0.5 text-[11px] font-medium text-text-primary">
                        <Check size={12} aria-hidden="true" />
                        {t('settings.engine.savedActive')}
                      </span>
                    )}
                  </span>
                  <span className="mt-1 block text-[12px] text-text-secondary leading-relaxed">
                    {t(`settings.engine.${mode}Description`)}
                  </span>
                  <span className="mt-3 block text-[12px] font-medium text-text-primary">
                    {t(`settings.engine.${mode}Route`)}
                  </span>
                  <span className="mt-1 block text-[12px] text-text-secondary">
                    {t(`settings.engine.${mode}Cost`)}
                  </span>
                </span>
              </span>
            </label>
          )
        })}
      </div>
      <p role="status" className="text-[12px] text-text-secondary">
        {pendingSwitch
          ? t('settings.engine.saveToSwitch')
          : savedMode === selectedMode
            ? t('settings.engine.nextRecording')
            : t('settings.engine.saveToApply')}
      </p>

      {selectedMode === 'existing' ? (
        <div className="rounded-[10px] border border-border bg-bg-secondary px-4 py-3 text-[12px] text-text-secondary space-y-2">
          <p className="font-medium text-text-primary">{t('settings.engine.existingConnection')}</p>
          <p>
            {t('settings.engine.currentStt')}: {config.stt_provider}
          </p>
          <p>
            {t('settings.engine.selectedDesktopModel')}: {config.desktop_polish_model}
          </p>
          <p>{t('settings.engine.existingNote')}</p>
        </div>
      ) : (
        <div className="space-y-3">
          <p className="text-[12px] text-text-secondary leading-relaxed">
            {t('settings.engine.openaiCharges')}
          </p>
          <OpenAiCredentialPanel />
        </div>
      )}
      <EngineConnectionTest
        key={`${config.engine_mode}:${config.desktop_polish_model}`}
        kind="stt"
        engineMode={config.engine_mode}
        desktopPolishModel={config.desktop_polish_model}
      />
    </div>
  )
}
