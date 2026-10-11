import { useTranslation } from 'react-i18next'
import i18n from '../../i18n'
import { invoke } from '@tauri-apps/api/core'
import { UI_LANGUAGES } from '../../lib/constants'
import { useAppStore } from '../../stores/appStore'
import titleUrl from '../../assets/brand/title-lockup.png'

export function WelcomeStep() {
  const { t } = useTranslation()
  const config = useAppStore((s) => s.config)
  const updateConfig = useAppStore((s) => s.updateConfig)

  const currentLang = config.ui_language || i18n.language || 'en'

  const handleSelectLanguage = (value: string) => {
    i18n.changeLanguage(value)
    localStorage.setItem('ui_language', value)
    updateConfig({ ui_language: value })
    invoke('refresh_tray_labels').catch(() => {})
  }

  return (
    <div className="space-y-6">
      <div className="py-4">
        <p className="term-eyebrow">system tray · windows &amp; macos</p>
        <img
          src={titleUrl}
          alt={t('app.name') + ' — ' + t('app.tagline')}
          className="w-full h-auto block mt-4"
        />
        <p className="text-[14px] text-text-secondary leading-[1.75] break-keep max-w-[52ch] mt-6 mx-auto text-center">
          {t('onboarding.speakToWrite')}
        </p>
        <div className="flex flex-wrap gap-2 mt-6 justify-center">
          <span className="term-chip">Whisper large-v3</span>
          <span className="term-chip">Codex cleanup</span>
          <span className="term-chip term-chip-muted">100% local</span>
        </div>
        <hr className="term-rule my-6" />
      </div>

      <div>
        <p className="text-[13px] font-medium text-text-secondary mb-3">
          {t('onboarding.selectLanguage')}
        </p>
        <div className="grid grid-cols-2 gap-3">
          {UI_LANGUAGES.map((lang) => (
            <button
              key={lang.value}
              onClick={() => handleSelectLanguage(lang.value)}
              className={`px-4 py-4 rounded-[10px] text-[14px] border cursor-pointer transition-all ${
                currentLang === lang.value
                  ? 'bg-accent/10 border-accent text-accent font-medium'
                  : 'bg-bg-secondary border-border text-text-primary hover:border-text-tertiary'
              }`}
            >
              <div className="font-medium">{lang.label}</div>
            </button>
          ))}
        </div>
        <p className="text-[12px] text-text-tertiary mt-3">{t('onboarding.selectLanguageDesc')}</p>
      </div>
    </div>
  )
}
