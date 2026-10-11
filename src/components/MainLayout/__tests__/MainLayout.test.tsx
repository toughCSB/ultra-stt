import React from 'react'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { MainLayout } from '../index'
import { useAppStore } from '../../../stores/appStore'

const MOTION_PROPS = new Set([
  'initial',
  'animate',
  'exit',
  'transition',
  'variants',
  'whileHover',
  'whileTap',
  'layoutId',
  'layout',
])

vi.mock('framer-motion', () => ({
  AnimatePresence: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  motion: new Proxy(
    {},
    {
      get:
        (_target, tag: string) =>
        ({ children, ...props }: React.HTMLAttributes<HTMLElement>) => {
          const domProps: Record<string, unknown> = {}
          for (const [key, value] of Object.entries(props)) {
            if (!MOTION_PROPS.has(key)) domProps[key] = value
          }
          return React.createElement(tag, domProps, children)
        },
    },
  ),
}))

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        'app.name': 'OpenTypeless',
        'app.tagline': 'AI Voice Input',
        'nav.home': 'Home',
        'nav.ask': 'Ask',
        'nav.settings': 'Settings',
        'nav.history': 'History',
        'nav.recovery': 'Input recovery',
        'recovery.pendingCount': '2 waiting',
        'nav.mainNavigation': 'Main navigation',
      })[key] ?? key,
  }),
}))

afterEach(() => {
  cleanup()
  window.location.hash = ''
})

describe('MainLayout', () => {
  it('shows the recovery item and pending count independently of history', () => {
    useAppStore.setState({
      config: { ...useAppStore.getState().config, history_enabled: false },
      recoveryEntries: [
        {
          id: 'r1',
          sessionId: 1,
          createdAt: '2026-10-03T00:00:00Z',
          text: 'one',
          appName: '',
          reason: 'output_failed',
          charsInserted: 0,
        },
        {
          id: 'r2',
          sessionId: 2,
          createdAt: '2026-10-03T00:00:01Z',
          text: 'two',
          appName: '',
          reason: 'partial_insert',
          charsInserted: 1,
        },
      ],
    })
    render(
      <MainLayout>
        <div>content</div>
      </MainLayout>,
    )
    expect(screen.getByRole('button', { name: 'Input recovery' })).toHaveTextContent('2')
    useAppStore.setState(useAppStore.getInitialState())
  })
  it('does not show Ask as a first-class navigation item', () => {
    render(
      <MainLayout>
        <div>content</div>
      </MainLayout>,
    )

    expect(screen.queryByRole('button', { name: 'Ask' })).not.toBeInTheDocument()
  })
})
