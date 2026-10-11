import { useState, useEffect, useCallback } from 'react'

export type Route = 'home' | 'settings' | 'history' | 'recovery'

export function parseHash(locationHash = window.location.hash): Route {
  const hash = locationHash.replace('#/', '')
  if (hash === 'history' || hash.startsWith('history?')) return 'history'
  if (hash === 'recovery') return 'recovery'
  if (hash === 'settings' || hash.startsWith('settings?')) return 'settings'
  return 'home'
}

function isValidDate(date: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) return false
  const parsed = new Date(`${date}T00:00:00Z`)
  return !Number.isNaN(parsed.getTime()) && parsed.toISOString().slice(0, 10) === date
}

export function parseHistoryDate(locationHash = window.location.hash): string | null {
  if (parseHash(locationHash) !== 'history') return null
  const query = locationHash.split('?')[1]
  const date = new URLSearchParams(query).get('date')
  return date && isValidDate(date) ? date : null
}

export function useRoute() {
  const [hash, setHash] = useState(window.location.hash)

  useEffect(() => {
    const onHashChange = () => setHash(window.location.hash)
    window.addEventListener('hashchange', onHashChange)
    return () => window.removeEventListener('hashchange', onHashChange)
  }, [])

  const navigate = useCallback((route: Route, options?: { date?: string }) => {
    const date = options?.date
    window.location.hash =
      route === 'home'
        ? '#/'
        : route === 'history' && date && isValidDate(date)
          ? `#/history?date=${date}`
          : `#/${route}`
  }, [])

  return { route: parseHash(hash), historyDate: parseHistoryDate(hash), navigate }
}
