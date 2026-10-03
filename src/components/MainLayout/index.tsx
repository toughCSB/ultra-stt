import { Home, Settings, History, ClipboardList } from 'lucide-react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { spring } from '../../lib/animations'
import { useRoute, type Route } from '../../lib/router'
import { BrandWordmark } from '../BrandWordmark'
import { useAppStore } from '../../stores/appStore'
import { AccessibilityBanner } from './AccessibilityBanner'

const baseNavItems: { id: Route; labelKey: string; icon: typeof Home }[] = [
  { id: 'home', labelKey: 'nav.home', icon: Home },
  { id: 'settings', labelKey: 'nav.settings', icon: Settings },
  { id: 'history', labelKey: 'nav.history', icon: History },
  { id: 'recovery', labelKey: 'nav.recovery', icon: ClipboardList },
]

interface Props {
  children: React.ReactNode
}

export function MainLayout({ children }: Props) {
  const { route, navigate } = useRoute()
  const { t } = useTranslation()
  const recoveryCount = useAppStore((state) => state.recoveryEntries.length)

  return (
    <div className="w-full h-full flex text-text-primary">
      {/* Sidebar — jelly surface */}
      <aside className="w-[236px] flex flex-col border-r border-border bg-bg-secondary/60 shrink-0">
        {/* Logo */}
        <div className="px-5 pt-5 pb-4" data-tauri-drag-region>
          <h1><BrandWordmark compact /></h1>
        </div>
        <div className="px-5 pb-4">
          <div className="flex items-center gap-2 text-[10px] uppercase tracking-[0.18em] text-text-tertiary">
            <span className="term-dot" />
            <span>local engine</span>
          </div>
        </div>
        {/* Main Nav */}
        <nav className="flex-1 px-3 space-y-0.5 relative" aria-label={t('nav.mainNavigation')}>
          {baseNavItems.map(({ id, labelKey, icon: Icon }) => {
            const active = route === id
            const label = t(labelKey)
            return (
              <motion.button
                key={id}
                onClick={() => navigate(id)}
                whileHover={{ scale: 1.03 }}
                whileTap={{ scaleX: 1.05, scaleY: 0.95 }}
                transition={spring.jellyGentle}
                aria-label={label}
                aria-current={active ? 'page' : undefined}
                className={`flex items-center gap-2.5 w-full px-3 py-2 text-[13px] rounded-[8px] transition-colors bg-transparent border-none cursor-pointer text-left relative ${
                  active
                    ? 'text-accent font-bold'
                    : 'text-text-secondary hover:text-text-primary'
                }`}
              >
                {active && (
                  <motion.div
                    layoutId="nav-indicator"
                    className="absolute inset-0 jelly-nav-active"
                    transition={spring.jellyGentle}
                  />
                )}
                <span className="relative z-10 flex items-center gap-2.5">
                  <Icon size={16} />
                  {label}
                  {id === 'recovery' && recoveryCount > 0 && (
                    <span className="ml-auto rounded-full bg-accent/15 px-1.5 py-0.5 text-[10px] tabular-nums text-accent" aria-label={t('recovery.pendingCount', { count: recoveryCount })}>
                      {recoveryCount}
                    </span>
                  )}
                </span>
              </motion.button>
            )
          })}

        </nav>

      </aside>

      {/* Content */}
      <main className="flex-1 min-w-0 flex flex-col">
        <AccessibilityBanner />
        <div className="flex-1 overflow-y-auto">{children}</div>
      </main>
    </div>
  )
}
