import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Zap } from 'lucide-react'
import { defaultLiveHotkey, hotkeyFromBinding, useAppStore } from '../stores/appStore'
import { updateConfig as persistConfig } from '../lib/tauri'
import { SegmentedControl } from './Settings/shared/SegmentedControl'

const IDLE_STOP_OPTIONS = ['0', '15', '30', '60', '120']

/** Home card for live typing: shortcuts plus the auto-stop time, saved immediately. */
export function LiveTypingCard() {
  const { t } = useTranslation()
  const config = useAppStore((s) => s.config)
  const setConfig = useAppStore((s) => s.setConfig)
  const setSavedConfig = useAppStore((s) => s.setSavedConfig)
  const applyPersistedConfigPatch = useAppStore((s) => s.applyPersistedConfigPatch)
  const [saveFailed, setSaveFailed] = useState(false)
  const firstLive = config.hotkeys.liveBindings?.[0]
  const liveHotkey = firstLive ? hotkeyFromBinding(firstLive) : defaultLiveHotkey()

  const saveIdleStop = async (seconds: number) => {
    const patch = { live_idle_stop_seconds: seconds }
    const previousConfig = useAppStore.getState().config
    const previousSavedConfig = useAppStore.getState().savedConfig
    const nextPersistedConfig = { ...(previousSavedConfig ?? previousConfig), ...patch }
    setConfig({ ...previousConfig, ...patch })
    setSaveFailed(false)
    try {
      await persistConfig(nextPersistedConfig)
      if (previousSavedConfig) applyPersistedConfigPatch(patch)
      else setSavedConfig(nextPersistedConfig)
    } catch {
      setConfig(previousConfig)
      setSaveFailed(true)
    }
  }

  return (
    <div className="jelly-card p-5 border border-orange-500/40">
      <div className="mb-3 flex items-center justify-between gap-3">
        <h3 className="flex items-center gap-2 text-[13px] font-semibold">
          <span className="flex h-6 w-6 items-center justify-center rounded-full bg-orange-500 text-white">
            <Zap size={13} strokeWidth={2.6} fill="currentColor" />
          </span>
          {t('settings.liveTyping')}
        </h3>
        <span className="rounded-full bg-orange-500/15 px-2 py-0.5 text-[11px] font-semibold text-orange-400">
          {liveHotkey}
        </span>
      </div>
      <p className="mb-3 text-[12px] leading-relaxed text-text-secondary break-keep">
        {t('settings.liveTypingHint', { liveHotkey, wordHotkey: 'Ctrl+Alt+W' })}
      </p>
      <p className="mb-1.5 text-[12px] text-text-tertiary">{t('settings.liveIdleStop')}</p>
      <SegmentedControl
        options={IDLE_STOP_OPTIONS.map((value) => ({
          value,
          label: value === '0' ? t('settings.liveIdleOff') : `${value}s`,
        }))}
        value={String(config.live_idle_stop_seconds)}
        onChange={(value) => void saveIdleStop(Number(value))}
      />
      {saveFailed && (
        <p className="mt-2 text-[12px] text-error">{t('settings.liveIdleSaveFailed')}</p>
      )}
    </div>
  )
}
