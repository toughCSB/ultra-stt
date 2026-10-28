import { useCallback, useEffect, useRef, useState } from 'react'
import { errorMessage } from '../lib/errors'
import { readCredential, setCredential } from '../lib/tauri'

interface StoredCredentialOptions {
  namespace: 'stt' | 'llm'
  provider: string
  /** False when the provider needs no key; the draft is then empty and nothing is saved. */
  enabled: boolean
  /** A key still kept in the settings file by older versions; it wins over the vault. */
  legacyKey: string
}

/**
 * A provider key kept in the system credential vault: loaded into an editable draft and
 * saved 350 ms after typing stops (or at once, with `persist(value, 0)`).
 */
export function useStoredCredential({
  namespace,
  provider,
  enabled,
  legacyKey,
}: StoredCredentialOptions) {
  const [draft, setDraft] = useState(legacyKey)
  const [saveError, setSaveError] = useState<string | null>(null)
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const label = namespace.toUpperCase()

  useEffect(() => {
    if (!enabled) {
      setDraft('')
      setSaveError(null)
      return
    }

    let cancelled = false
    setDraft(legacyKey)
    setSaveError(null)
    readCredential(namespace, provider)
      .then((secret) => {
        if (!cancelled) setDraft(legacyKey || secret || '')
      })
      .catch((error) => console.error(`[credentials] failed to read ${label} credential`, error))

    return () => {
      cancelled = true
    }
  }, [enabled, label, legacyKey, namespace, provider])

  const persist = useCallback(
    (value: string, delayMs = 350) => {
      if (!enabled) return
      if (saveTimer.current) clearTimeout(saveTimer.current)
      saveTimer.current = setTimeout(() => {
        saveTimer.current = null
        setCredential(namespace, provider, value)
          .then(() => setSaveError(null))
          .catch((error) => {
            setSaveError(errorMessage(error))
            console.error(`[credentials] failed to save ${label} credential`, error)
          })
      }, delayMs)
    },
    [enabled, label, namespace, provider],
  )

  return { draft, setDraft, saveError, setSaveError, persist }
}
