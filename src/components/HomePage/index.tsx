import { Settings, History } from 'lucide-react'
import titleUrl from '../../assets/brand/title-lockup.png'
import { motion } from 'framer-motion'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { spring } from '../../lib/animations'
import { useAppStore } from '../../stores/appStore'
import { useRoute } from '../../lib/router'
import { PolishModeSelector } from '../PolishModeSelector'

function localDateKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`
}

export function HomePage() {
  const config = useAppStore((s) => s.config)
  const history = useAppStore((s) => s.history)
  const lastPipelineTiming = useAppStore((s) => s.lastPipelineTiming)
  const { navigate } = useRoute()
  const { t } = useTranslation()

  const today = localDateKey(new Date())
  const activity = useMemo(() => {
    const [year, month, day] = today.split('-').map(Number)
    const counts = new Map<string, number>()
    for (const entry of history) {
      const date = entry.created_at.slice(0, 10)
      counts.set(date, (counts.get(date) ?? 0) + 1)
    }
    return Array.from({ length: 14 }, (_, index) => {
      const date = new Date(year, month - 1, day - 13 + index)
      const key = localDateKey(date)
      return { date: key, count: counts.get(key) ?? 0 }
    })
  }, [history, today])
  const todayCount = activity[13].count
  const maxCount = Math.max(1, ...activity.map((day) => day.count))
  const isUltraRemote =
    config.stt_provider === 'custom-whisper' && config.stt_custom_preset === 'ultra-remote'
  const timingItems: Array<[string, number]> = lastPipelineTiming
    ? isUltraRemote
      ? [
          ['remoteProcessingTime', lastPipelineTiming.stt_ms],
          ['totalTime', lastPipelineTiming.total_ms],
        ]
      : [
          ['sttTime', lastPipelineTiming.stt_ms],
          ['polishTime', lastPipelineTiming.llm_ms],
          ['totalTime', lastPipelineTiming.total_ms],
        ]
    : []
  return (
    <div className="p-6 space-y-6">
      {/* Hero */}
      <div className="jelly-card overflow-hidden">
        <img
          src={titleUrl}
          alt={t('app.name') + ' — ' + t('app.tagline')}
          className="w-full h-auto block"
        />
        <div className="px-5 py-5">
          <p className="term-eyebrow">system tray · windows &amp; macos</p>
          <p className="text-[14px] leading-[1.75] text-text-secondary mt-3 break-keep max-w-[62ch]">
            {t('home.description', { hotkey: config.hotkey })}
          </p>
          <div className="flex flex-wrap gap-2 mt-4">
            <span className="term-chip">Whisper large-v3</span>
            <span className="term-chip">Codex cleanup</span>
            <span className="term-chip term-chip-muted">{t('home.welcome')}</span>
          </div>
        </div>
      </div>

      {/* Readout */}
      <div className="jelly-card p-5">
        <div className="mb-3 flex items-center justify-between gap-3">
          <h3 className="text-[13px] font-semibold">{t('settings.polishModeTitle')}</h3>
          <span className="text-[11px] text-accent">
            {config.polish_enabled ? t('settings.polishModeSelected') : t('home.disabled')}
          </span>
        </div>
        <PolishModeSelector />
      </div>

      <div className="jelly-card p-5">
        <div className="flex items-center justify-between mb-4">
          <span className="text-[11px] uppercase tracking-[0.2em] text-text-secondary">
            {t('home.today')}
          </span>
          <span className="flex items-center gap-2 text-[11px] uppercase tracking-[0.18em] text-accent">
            <span className="term-dot" />
            live
          </span>
        </div>
        <p className="term-figure text-[44px] leading-none">{todayCount}</p>
        <p className="text-[12px] text-text-tertiary mt-2">
          {t('home.loadedRecordings', { count: history.length })}
        </p>
        <div className="mt-5 flex items-center justify-between gap-2 text-[11px] text-text-secondary">
          <span>{t('home.recentActivity')}</span>
          <span>{t('home.loadedHistoryNotice', { count: history.length })}</span>
        </div>
        <div className="mt-2 flex gap-1" role="group" aria-label={t('home.recentActivity')}>
          {activity.map(({ date, count }) => {
            const label = t('home.activityOnDate', { date, count })
            return (
              <button
                key={date}
                type="button"
                onClick={() => navigate('history', { date })}
                aria-label={label}
                title={label}
                className="flex min-w-0 flex-1 flex-col items-center gap-1 rounded-[4px] bg-transparent px-0.5 text-text-tertiary hover:bg-bg-tertiary focus-visible:outline-2 focus-visible:outline-accent"
              >
                <span className="text-[10px] tabular-nums">{count}</span>
                <span className="flex h-12 w-full items-end justify-center">
                  <span
                    className={`w-full max-w-7 rounded-t-[2px] ${count === 0 ? 'bg-bg-tertiary' : date === today ? 'bg-accent' : 'bg-accent/55'}`}
                    style={{
                      height: count === 0 ? '2px' : `${Math.max(15, (count / maxCount) * 100)}%`,
                    }}
                  />
                </span>
                <span className="text-[9px] tabular-nums">{date.slice(5).replace('-', '/')}</span>
              </button>
            )
          })}
        </div>
      </div>

      {lastPipelineTiming && (
        <div className="jelly-card p-5">
          <h3 className="text-[13px] font-medium mb-3">{t('home.lastProcessingTime')}</h3>
          <div
            className={`grid gap-3 text-center ${isUltraRemote ? 'grid-cols-2' : 'grid-cols-3'}`}
          >
            {timingItems.map(([label, milliseconds]) => (
              <div key={label} className="rounded-[10px] bg-bg-secondary p-2">
                <p className="text-[11px] text-text-tertiary">{t(`home.${label}`)}</p>
                <p className="mt-1 text-[16px] font-medium tabular-nums">
                  {t('home.seconds', { value: (milliseconds / 1000).toFixed(1) })}
                </p>
              </div>
            ))}
          </div>
          {isUltraRemote && (
            <p className="mt-2 text-[11px] text-text-tertiary">{t('home.remoteTimingNote')}</p>
          )}
        </div>
      )}

      {/* Current config */}
      <div className="rounded-[18px] p-5 jelly-card">
        <h3 className="text-[13px] font-medium mb-3">{t('home.currentConfig')}</h3>
        <div className="space-y-2 text-[13px]">
          <div className="flex justify-between">
            <span className="text-text-secondary">{t('home.sttProvider')}</span>
            <span className="text-text-primary font-medium">{config.stt_provider}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-secondary">{t('home.llmProvider')}</span>
            <span className="text-text-primary font-medium">{config.llm_provider}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-secondary">{t('home.aiPolish')}</span>
            <span className="text-text-primary font-medium">
              {config.polish_enabled ? t('home.enabled') : t('home.disabled')}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-secondary">{t('home.outputMode')}</span>
            <span className="text-text-primary font-medium">{config.output_mode}</span>
          </div>
        </div>
      </div>

      {/* Quick actions */}
      <div className="grid grid-cols-2 gap-3">
        <motion.button
          onClick={() => navigate('settings')}
          whileHover={{ scale: 1.04 }}
          whileTap={{ scaleX: 1.06, scaleY: 0.94 }}
          transition={spring.jellyGentle}
          className="flex items-center gap-2.5 rounded-[14px] p-4 cursor-pointer text-left jelly-btn"
        >
          <Settings size={16} className="text-text-secondary" />
          <span className="text-[13px] font-medium">{t('nav.settings')}</span>
        </motion.button>
        <motion.button
          onClick={() => navigate('history')}
          whileHover={{ scale: 1.04 }}
          whileTap={{ scaleX: 1.06, scaleY: 0.94 }}
          transition={spring.jellyGentle}
          className="flex items-center gap-2.5 rounded-[14px] p-4 cursor-pointer text-left jelly-btn"
        >
          <History size={16} className="text-text-secondary" />
          <span className="text-[13px] font-medium">{t('nav.history')}</span>
        </motion.button>
      </div>
    </div>
  )
}
