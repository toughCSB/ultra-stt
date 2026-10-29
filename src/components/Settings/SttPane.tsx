import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useAppStore } from '../../stores/appStore'
import { LANGUAGES } from '../../lib/constants'
import { getSttRecordingCapability, type ResolvedSttRecordingLimit } from '../../lib/tauri'
import { FormField } from './shared/FormField'

const RECORDING_LIMIT_PRESETS = [30, 60, 120, 300, 600, 1800, 3600]
const MIN_CUSTOM_RECORDING_SECONDS = 30

function formatRecordingDuration(
  seconds: number,
  t: (key: string, values?: Record<string, number>) => string,
) {
  if (seconds === 60) return t('recordingLimits.durationMinute', { count: 1 })
  if (seconds > 0 && seconds % 60 === 0) {
    return t('recordingLimits.durationMinutes', { count: seconds / 60 })
  }
  return t('recordingLimits.durationSeconds', { count: seconds })
}

export function SttPane() {
  const config = useAppStore((state) => state.config)
  const updateConfig = useAppStore((state) => state.updateConfig)
  const { t } = useTranslation()
  const [recordingLimit, setRecordingLimit] = useState<ResolvedSttRecordingLimit | null>(null)
  const [customDurationEntryRequested, setCustomDurationEntryRequested] = useState(false)

  useEffect(() => {
    let cancelled = false
    setRecordingLimit(null)
    getSttRecordingCapability(
      config.engine_mode === 'openai' ? 'openai-transcribe' : config.stt_provider,
      config.recording_limit_mode,
      config.custom_recording_limit_seconds,
    )
      .then((resolved) => {
        if (!cancelled) setRecordingLimit(resolved)
      })
      .catch((error) => {
        console.error('[stt] failed to resolve recording limit', error)
        if (!cancelled) setRecordingLimit(null)
      })

    return () => {
      cancelled = true
    }
  }, [
    config.custom_recording_limit_seconds,
    config.max_recording_seconds,
    config.recording_limit_mode,
    config.engine_mode,
    config.stt_provider,
  ])

  useEffect(() => {
    setCustomDurationEntryRequested(false)
  }, [config.stt_provider])

  useEffect(() => {
    if (config.recording_limit_mode === 'auto') {
      setCustomDurationEntryRequested(false)
    }
  }, [config.recording_limit_mode])

  const availableRecordingPresets = recordingLimit
    ? RECORDING_LIMIT_PRESETS.filter(
        (seconds) => seconds <= recordingLimit.capability.hardMaxSeconds,
      )
    : []
  const savedDurationIsPreset =
    recordingLimit !== null &&
    availableRecordingPresets.includes(config.custom_recording_limit_seconds)
  const canEnterCustomDuration =
    recordingLimit !== null &&
    recordingLimit.capability.hardMaxSeconds > MIN_CUSTOM_RECORDING_SECONDS
  const recordingDurationValue =
    config.recording_limit_mode === 'auto'
      ? 'auto'
      : customDurationEntryRequested && canEnterCustomDuration
        ? 'custom'
        : savedDurationIsPreset
          ? String(config.custom_recording_limit_seconds)
          : canEnterCustomDuration
            ? 'custom'
            : String(recordingLimit?.effectiveMaxSeconds ?? MIN_CUSTOM_RECORDING_SECONDS)
  const showCustomDurationEntry =
    canEnterCustomDuration &&
    (customDurationEntryRequested ||
      (config.recording_limit_mode === 'custom' && !savedDurationIsPreset))
  const isManagedCapability =
    recordingLimit?.capability.explanationKey === 'recordingLimits.reasons.managedCapability'
  const isManagedFallback =
    recordingLimit?.capability.explanationKey === 'recordingLimits.reasons.managedFallback'
  const isProviderFixedLimit =
    !canEnterCustomDuration &&
    recordingLimit?.capability.explanationKey === 'recordingLimits.reasons.providerDuration'
  const recordingLimitHelper = recordingLimit
    ? showCustomDurationEntry
      ? t(
          isManagedCapability
            ? 'recordingLimits.allowedCloudRange'
            : 'recordingLimits.allowedRangeWithReason',
          {
            min: formatRecordingDuration(MIN_CUSTOM_RECORDING_SECONDS, t),
            max: formatRecordingDuration(recordingLimit.capability.hardMaxSeconds, t),
            reason: t(recordingLimit.capability.explanationKey),
          },
        )
      : config.recording_limit_mode === 'custom'
        ? isProviderFixedLimit
          ? t('recordingLimits.providerFixedLimit', {
              max: formatRecordingDuration(recordingLimit.capability.hardMaxSeconds, t),
            })
          : isManagedCapability
            ? t('recordingLimits.currentSelectionWithCloudMax', {
                current: formatRecordingDuration(recordingLimit.effectiveMaxSeconds, t),
                max: formatRecordingDuration(recordingLimit.capability.hardMaxSeconds, t),
              })
            : isManagedFallback
              ? t(recordingLimit.capability.explanationKey)
              : t('recordingLimits.currentSelectionWithLimit', {
                  current: formatRecordingDuration(recordingLimit.effectiveMaxSeconds, t),
                  max: formatRecordingDuration(recordingLimit.capability.hardMaxSeconds, t),
                  reason: t(recordingLimit.capability.explanationKey),
                })
        : t(recordingLimit.capability.explanationKey)
    : null

  const handleRecordingDurationChange = (value: string) => {
    if (value === 'auto') {
      setCustomDurationEntryRequested(false)
      updateConfig({ recording_limit_mode: 'auto' })
      return
    }
    if (value === 'custom') {
      setCustomDurationEntryRequested(true)
      updateConfig({ recording_limit_mode: 'custom' })
      return
    }

    const seconds = Number(value)
    if (!Number.isFinite(seconds)) return
    setCustomDurationEntryRequested(false)
    updateConfig({
      recording_limit_mode: 'custom',
      custom_recording_limit_seconds: seconds,
    })
  }

  return (
    <div className="space-y-5">
      <FormField label={t('settings.sttLanguage')}>
        <select
          value={config.stt_language}
          onChange={(e) => updateConfig({ stt_language: e.target.value })}
          className="w-full px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
        >
          {LANGUAGES.map((l) => (
            <option key={l.value} value={l.value}>
              {l.labelKey ? t(l.labelKey) : l.label}
            </option>
          ))}
        </select>
      </FormField>

      <FormField label={t('settings.maxRecordingDuration')}>
        {recordingLimit ? (
          <div className="space-y-2">
            <select
              aria-label={t('settings.maxRecordingDuration')}
              value={recordingDurationValue}
              onChange={(event) => handleRecordingDurationChange(event.target.value)}
              className="w-full px-3 py-2.5 bg-bg-secondary border border-border rounded-[10px] text-[13px] text-text-primary outline-none focus:border-border-focus transition-colors"
            >
              <option value="auto">
                {t('recordingLimits.auto', {
                  duration: formatRecordingDuration(
                    recordingLimit.capability.recommendedMaxSeconds,
                    t,
                  ),
                })}
              </option>
              {availableRecordingPresets.map((seconds) => (
                <option key={seconds} value={seconds}>
                  {formatRecordingDuration(seconds, t)}
                </option>
              ))}
              {canEnterCustomDuration && (
                <option value="custom">{t('recordingLimits.numericEntry')}</option>
              )}
            </select>

            {showCustomDurationEntry && (
              <div className="space-y-1.5">
                <label
                  htmlFor="custom-recording-duration"
                  className="block text-[12px] font-medium text-text-secondary"
                >
                  {t('recordingLimits.customDuration')}
                </label>
                <div className="flex items-center rounded-[10px] border border-border bg-bg-secondary transition-colors focus-within:border-border-focus">
                  <input
                    id="custom-recording-duration"
                    aria-label={t('recordingLimits.customDuration')}
                    type="number"
                    min={MIN_CUSTOM_RECORDING_SECONDS}
                    max={recordingLimit.capability.hardMaxSeconds}
                    step={1}
                    value={config.custom_recording_limit_seconds}
                    onChange={(event) => {
                      const seconds = Number(event.target.value)
                      if (Number.isFinite(seconds) && seconds >= 0) {
                        updateConfig({ custom_recording_limit_seconds: Math.floor(seconds) })
                      }
                    }}
                    className="min-w-0 flex-1 bg-transparent px-3 py-2.5 text-[13px] text-text-primary outline-none"
                  />
                  <span className="pr-3 text-[12px] text-text-secondary" aria-hidden="true">
                    {t('recordingLimits.secondsUnit')}
                  </span>
                </div>
              </div>
            )}

            {recordingLimitHelper && (
              <p className="text-[12px] text-text-secondary">{recordingLimitHelper}</p>
            )}
            {config.recording_limit_mode === 'custom' &&
              recordingLimit.requestedSeconds !== recordingLimit.effectiveMaxSeconds && (
                <p className="text-[12px] text-amber-500">
                  {t('recordingLimits.corrected', {
                    duration: formatRecordingDuration(recordingLimit.effectiveMaxSeconds, t),
                  })}
                </p>
              )}
          </div>
        ) : (
          <p className="text-[12px] text-text-tertiary">{t('recordingLimits.loading')}</p>
        )}
      </FormField>
    </div>
  )
}
