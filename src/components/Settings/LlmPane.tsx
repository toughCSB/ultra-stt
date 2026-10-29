import { useState, useEffect, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import type { PolishStyle } from '../../stores/appStore'
import { getLatestMappingCandidate, listCustomAppMappings } from '../../lib/tauri'
import type { CustomAppMappingView, MappingCandidateView } from '../../lib/tauri'
import { FormField } from './shared/FormField'
import { Toggle } from './shared/Toggle'
import { ChevronDown, MoreHorizontal } from 'lucide-react'
import { AppLogo } from '../AppLogo'
import { DESKTOP_POLISH_MODELS, isDesktopPolishModel } from '../../lib/desktop-polish-models'
import { EngineConnectionTest } from './EngineConnectionTest'
import { ContextAdaptationApps } from './ContextAdaptationApps'
import { TranslationTargets } from './TranslationTargets'
import { AppStyleMappingDialog } from './AppStyleMappingDialog'
import { ManageAppMappingsDialog } from './ManageAppMappingsDialog'

export function LlmPane() {
  const config = useAppStore((state) => state.config)
  const updateConfig = useAppStore((state) => state.updateConfig)
  const lastContext = useAppStore((state) => state.lastContext)
  const { t } = useTranslation()
  const polishPromptLength = config.polish_custom_prompt.length
  const hasCustomPolishConfig = config.polish_custom_prompt.trim().length > 0
  const [polishAdvancedOpen, setPolishAdvancedOpen] = useState(hasCustomPolishConfig)
  const [mappingCandidate, setMappingCandidate] = useState<MappingCandidateView | null>(null)
  const [appMappings, setAppMappings] = useState<CustomAppMappingView[]>([])
  const [appStyleMenuOpen, setAppStyleMenuOpen] = useState(false)
  const [appStyleDialogOpen, setAppStyleDialogOpen] = useState(false)
  const [manageMappingsOpen, setManageMappingsOpen] = useState(false)
  const [editingMapping, setEditingMapping] = useState<CustomAppMappingView | null>(null)
  const appStyleMenuButtonRef = useRef<HTMLButtonElement>(null)
  const showBrowserAccessHint = Boolean(
    config.polish_enabled &&
    config.context_adaptation_enabled &&
    lastContext?.profileId === 'general.browser' &&
    lastContext.browserAccessStatus === 'needs_permission',
  )
  const selectedModel = isDesktopPolishModel(config.desktop_polish_model)
    ? config.desktop_polish_model
    : 'gpt-6-luna'
  const openAiModel = selectedModel.startsWith('gpt-')
  const grokPending = selectedModel === 'grok-4.7'
  const cliProvider = selectedModel.startsWith('claude-') ? 'Claude' : 'Grok'

  const refreshAppMappings = useCallback(async () => {
    const [candidate, mappings] = await Promise.all([
      getLatestMappingCandidate(),
      listCustomAppMappings(),
    ])
    setMappingCandidate(candidate)
    setAppMappings(mappings)
  }, [])

  useEffect(() => {
    if (!lastContext) {
      setMappingCandidate(null)
      setAppMappings([])
      return
    }
    let cancelled = false
    Promise.all([getLatestMappingCandidate(), listCustomAppMappings()])
      .then(([candidate, mappings]) => {
        if (cancelled) return
        setMappingCandidate(candidate)
        setAppMappings(mappings)
      })
      .catch(() => {
        if (cancelled) return
        setMappingCandidate(null)
        setAppMappings([])
      })
    return () => {
      cancelled = true
    }
  }, [lastContext])

  const closeAppStyleMenu = useCallback((restoreFocus = false) => {
    setAppStyleMenuOpen(false)
    if (restoreFocus) requestAnimationFrame(() => appStyleMenuButtonRef.current?.focus())
  }, [])

  useEffect(() => {
    if (!appStyleMenuOpen) return
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.preventDefault()
      closeAppStyleMenu(true)
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [appStyleMenuOpen, closeAppStyleMenu])

  useEffect(() => {
    if (hasCustomPolishConfig) setPolishAdvancedOpen(true)
  }, [hasCustomPolishConfig])

  return (
    <div className="space-y-4">
      <FormField label={t('settings.engine.desktopPolishModel')}>
        <select
          aria-label={t('settings.engine.desktopPolishModel')}
          value={selectedModel}
          onChange={(event) => {
            if (event.target.value !== 'grok-4.7' && isDesktopPolishModel(event.target.value)) {
              updateConfig({ desktop_polish_model: event.target.value })
            }
          }}
          className="w-full px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
        >
          {DESKTOP_POLISH_MODELS.map((model) => (
            <option key={model} value={model} disabled={model === 'grok-4.7'}>
              {model === 'grok-4.7' ? t('settings.engine.grokPendingOption') : model}
            </option>
          ))}
        </select>
        <p className="mt-2 text-[12px] leading-relaxed text-text-secondary">
          {grokPending
            ? t('settings.engine.grokPendingNote')
            : openAiModel
              ? t(
                  config.engine_mode === 'openai'
                    ? 'settings.engine.openAiApiRoute'
                    : 'settings.engine.codexCliRoute',
                )
              : t('settings.engine.localCliRoute', { provider: cliProvider })}
        </p>
        <p className="mt-1 text-[12px] leading-relaxed text-text-secondary">
          {t('settings.engine.desktopRoutingNote')}
        </p>
      </FormField>
      <EngineConnectionTest
        key={`${config.engine_mode}:${selectedModel}`}
        kind="llm"
        engineMode={config.engine_mode}
        desktopPolishModel={selectedModel}
        disabled={grokPending}
      />
      <div className="space-y-3 pt-1">
        <div>
          <Toggle
            checked={config.polish_enabled}
            onChange={(checked) => updateConfig({ polish_enabled: checked })}
            label={t('settings.enableAiPolish')}
          />
        </div>
        <div>
          <Toggle
            checked={config.context_adaptation_enabled}
            disabled={!config.polish_enabled}
            onChange={(checked) => updateConfig({ context_adaptation_enabled: checked })}
            label={t('settings.contextAdaptation')}
          />
          <ContextAdaptationApps
            disabled={!config.polish_enabled || !config.context_adaptation_enabled}
          />
          {lastContext && (
            <div className="mt-2 ml-[52px] min-w-0">
              <p className="text-[11px] leading-relaxed text-text-tertiary">
                {t('settings.lastDictationContext')}
              </p>
              <div className="mt-1 flex min-w-0 items-center gap-1.5 text-[12px] text-text-secondary">
                <AppLogo iconKey={lastContext.iconKey} family={lastContext.family} />
                <span className="min-w-0 truncate">{lastContext.appLabel}</span>
                {(mappingCandidate || appMappings.length > 0) && (
                  <div className="relative ml-auto flex-none">
                    <button
                      ref={appStyleMenuButtonRef}
                      type="button"
                      aria-label={t('settings.appStyleMenu')}
                      title={t('settings.appStyleMenu')}
                      aria-expanded={appStyleMenuOpen}
                      onClick={() => setAppStyleMenuOpen((open) => !open)}
                      className="grid h-7 w-7 place-items-center rounded-[6px] border-none bg-transparent text-text-tertiary hover:bg-bg-hover hover:text-text-primary"
                    >
                      <MoreHorizontal size={15} />
                    </button>
                    {appStyleMenuOpen && (
                      <>
                        <div
                          className="fixed inset-0 z-30"
                          onClick={() => closeAppStyleMenu(true)}
                        />
                        <div className="absolute right-0 top-8 z-40 min-w-[210px] rounded-[8px] border border-border bg-bg-primary py-1 shadow-float">
                          {mappingCandidate && (
                            <button
                              type="button"
                              onClick={() => {
                                closeAppStyleMenu()
                                setEditingMapping(null)
                                setAppStyleDialogOpen(true)
                              }}
                              className="block w-full px-3 py-2 text-left text-[12px] text-text-secondary hover:bg-bg-hover hover:text-text-primary"
                            >
                              {t('settings.useDifferentWritingStyle')}
                            </button>
                          )}
                          {appMappings.length > 0 && (
                            <button
                              type="button"
                              onClick={() => {
                                closeAppStyleMenu()
                                setManageMappingsOpen(true)
                              }}
                              className="block w-full px-3 py-2 text-left text-[12px] text-text-secondary hover:bg-bg-hover hover:text-text-primary"
                            >
                              {t('settings.manageAppMappings')}
                            </button>
                          )}
                        </div>
                      </>
                    )}
                  </div>
                )}
              </div>
              {showBrowserAccessHint && (
                <p className="mt-1 text-[11px] leading-relaxed text-amber-600">
                  {t('settings.browserAccessHint')}
                </p>
              )}
            </div>
          )}
        </div>
        <div>
          <Toggle
            checked={config.translate_enabled}
            onChange={(checked) => updateConfig({ translate_enabled: checked })}
            label={t('settings.translationMode')}
          />
          {config.translate_enabled && (
            <div className="mt-2 ml-[52px] min-w-0">
              <TranslationTargets
                value={config.translation}
                onChange={(translation) => updateConfig({ translation })}
              />
            </div>
          )}
        </div>
      </div>

      {config.polish_enabled && (
        <FormField label={t('settings.polishStyle')}>
          <select
            value={config.polish_style}
            onChange={(e) => updateConfig({ polish_style: e.target.value as PolishStyle })}
            className="w-full px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
          >
            <option value="original">{t('settings.polishStyleOriginal')}</option>
            <option value="minimal">{t('settings.polishStyleMinimal')}</option>
            <option value="clean">{t('settings.polishStyleClean')}</option>
            <option value="structured">{t('settings.polishStyleStructured')}</option>
            <option value="professional">{t('settings.polishStyleProfessional')}</option>
          </select>
          {config.polish_style === 'original' && (
            <p className="mt-2 text-[11px] leading-relaxed text-text-tertiary">
              {t('settings.polishStyleOriginalHint')}
            </p>
          )}
        </FormField>
      )}

      <div>
        <button
          type="button"
          aria-expanded={polishAdvancedOpen}
          onClick={() => setPolishAdvancedOpen((open) => !open)}
          className="flex w-full items-center justify-between rounded-[10px] border border-border bg-bg-secondary/40 px-3 py-2 text-left text-[13px] font-medium text-text-primary transition-colors hover:border-border-focus"
        >
          <span>{t('settings.advancedPolishSettings')}</span>
          <ChevronDown
            size={14}
            className={`text-text-tertiary transition-transform ${
              polishAdvancedOpen ? 'rotate-180' : ''
            }`}
          />
        </button>

        {polishAdvancedOpen && (
          <div className="mt-3 space-y-3">
            <div>
              <Toggle
                checked={config.selected_text_enabled}
                onChange={(checked) => updateConfig({ selected_text_enabled: checked })}
                label={t('settings.selectedTextContext')}
              />
              <p className="mt-1 ml-[52px] text-[11px] leading-relaxed text-text-tertiary">
                {t('settings.selectedTextContextDesc')}
              </p>
            </div>

            {config.polish_enabled && (
              <FormField label={t('settings.customPolishInstructions')}>
                <textarea
                  value={config.polish_custom_prompt}
                  onChange={(e) => updateConfig({ polish_custom_prompt: e.target.value })}
                  maxLength={2000}
                  rows={4}
                  placeholder={t('settings.customPolishInstructionsPlaceholder')}
                  className="w-full resize-y px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
                />
                <p className="text-[11px] text-text-tertiary mt-1.5">
                  {t('settings.customPolishInstructionsCount', { count: polishPromptLength })}
                </p>
              </FormField>
            )}
          </div>
        )}
      </div>

      {appStyleDialogOpen && lastContext && (
        <AppStyleMappingDialog
          candidate={editingMapping ? null : mappingCandidate}
          mapping={editingMapping}
          context={lastContext}
          config={config}
          onCancel={() => {
            setAppStyleDialogOpen(false)
            setEditingMapping(null)
          }}
          onSaved={async () => {
            await refreshAppMappings()
            setAppStyleDialogOpen(false)
            setEditingMapping(null)
          }}
        />
      )}

      {manageMappingsOpen && (
        <ManageAppMappingsDialog
          mappings={appMappings}
          onCancel={() => setManageMappingsOpen(false)}
          onChanged={refreshAppMappings}
          onEdit={(mapping) => {
            setManageMappingsOpen(false)
            setEditingMapping(mapping)
            setAppStyleDialogOpen(true)
          }}
        />
      )}
    </div>
  )
}
