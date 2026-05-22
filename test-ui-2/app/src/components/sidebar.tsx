import * as React from 'react'
import { AnimatePresence, motion } from 'motion/react'
import {
  Activity,
  AlertTriangle,
  BarChart3,
  Cpu,
  LayoutDashboard,
  PanelLeft,
  Settings,
  Sparkles,
  Workflow,
  X,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

type NavItem = {
  id: string
  label: string
  icon: React.ComponentType<{ className?: string; 'aria-hidden'?: boolean }>
  badge?: string
}

const NAV: { group: string; items: NavItem[] }[] = [
  {
    group: 'Operations',
    items: [
      { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard },
      { id: 'showcase', label: 'Showcase', icon: Sparkles },
      { id: 'devices', label: 'Devices', icon: Cpu, badge: '6' },
      { id: 'alerts', label: 'Alerts', icon: AlertTriangle, badge: '3' },
      { id: 'analytics', label: 'Analytics', icon: BarChart3 },
    ],
  },
  {
    group: 'Control',
    items: [
      { id: 'automations', label: 'Automations', icon: Workflow },
      { id: 'logs', label: 'Activity Log', icon: Activity },
      { id: 'settings', label: 'Settings', icon: Settings },
    ],
  },
]

type Ctx = {
  collapsed: boolean
  setCollapsed: (v: boolean) => void
  mobileOpen: boolean
  setMobileOpen: (v: boolean) => void
  active: string
  setActive: (v: string) => void
}

const SidebarCtx = React.createContext<Ctx | null>(null)

export function useSidebar() {
  const ctx = React.useContext(SidebarCtx)
  if (!ctx) throw new Error('useSidebar must be used inside <SidebarProvider>')
  return ctx
}

const COOKIE = 'iot_sidebar_collapsed'

function readCookie(): boolean {
  if (typeof document === 'undefined') return false
  const m = document.cookie.match(new RegExp(`(?:^|; )${COOKIE}=([^;]*)`))
  return m?.[1] === '1'
}

function writeCookie(v: boolean) {
  if (typeof document === 'undefined') return
  document.cookie = `${COOKIE}=${v ? '1' : '0'}; path=/; max-age=${60 * 60 * 24 * 30}`
}

export function SidebarProvider({ children }: { children: React.ReactNode }) {
  const [collapsed, setCollapsedState] = React.useState<boolean>(() => readCookie())
  const [mobileOpen, setMobileOpen] = React.useState(false)
  const [active, setActive] = React.useState('dashboard')

  const setCollapsed = React.useCallback((v: boolean) => {
    setCollapsedState(v)
    writeCookie(v)
  }, [])

  // Cmd/Ctrl + B to toggle
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault()
        setCollapsed(!collapsed)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [collapsed, setCollapsed])

  return (
    <SidebarCtx.Provider
      value={{ collapsed, setCollapsed, mobileOpen, setMobileOpen, active, setActive }}
    >
      {children}
    </SidebarCtx.Provider>
  )
}

function SidebarBody({ onItemClick }: { onItemClick?: () => void }) {
  const { collapsed, active, setActive } = useSidebar()

  return (
    <div className="flex h-full flex-col">
      {/* Brand */}
      <div className={cn('flex items-center gap-2 px-3 py-3', collapsed && 'justify-center px-2')}>
        <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-[var(--color-cta)]/15 text-[var(--color-cta)] ring-1 ring-inset ring-[var(--color-cta)]/30">
          <Activity className="size-5" aria-hidden />
        </div>
        {!collapsed && (
          <div className="min-w-0 leading-tight">
            <div
              className="truncate font-mono text-sm font-semibold"
              style={{ textShadow: '0 0 8px rgba(34,197,94,0.25)' }}
            >
              IoT Control
            </div>
            <div className="truncate font-mono text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
              edge fleet · v2.14
            </div>
          </div>
        )}
      </div>

      <div className="mx-3 my-1 h-px bg-[var(--color-border)]" />

      {/* Nav groups */}
      <nav className="flex-1 overflow-y-auto px-2 py-2">
        {NAV.map((group) => (
          <div key={group.group} className="mb-4">
            {!collapsed && (
              <div className="px-2 pb-1 font-mono text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
                {group.group}
              </div>
            )}
            <ul className="space-y-0.5">
              {group.items.map((item) => {
                const Icon = item.icon
                const isActive = active === item.id
                return (
                  <li key={item.id}>
                    <button
                      type="button"
                      title={collapsed ? item.label : undefined}
                      aria-current={isActive ? 'page' : undefined}
                      onClick={() => {
                        setActive(item.id)
                        onItemClick?.()
                      }}
                      className={cn(
                        'group relative flex w-full cursor-pointer items-center gap-3 rounded-md px-2 py-2 text-sm transition-colors duration-150',
                        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-cta)]',
                        isActive
                          ? 'bg-[var(--color-cta)]/12 text-[var(--color-text)]'
                          : 'text-[var(--color-muted)] hover:bg-[var(--color-surface-2)]/60 hover:text-[var(--color-text)]',
                        collapsed && 'justify-center px-0',
                      )}
                    >
                      {isActive && (
                        <motion.span
                          layoutId="sidebar-active-indicator"
                          aria-hidden
                          className="absolute inset-y-1 left-0 w-0.5 rounded-r bg-[var(--color-cta)]"
                          style={{ boxShadow: '0 0 8px var(--color-cta)' }}
                          transition={{ type: 'spring', stiffness: 400, damping: 32 }}
                        />
                      )}
                      <Icon
                        className={cn(
                          'size-4 shrink-0',
                          isActive ? 'text-[var(--color-cta)]' : 'text-current',
                        )}
                        aria-hidden
                      />
                      {!collapsed && <span className="flex-1 truncate text-left">{item.label}</span>}
                      {!collapsed && item.badge && (
                        <span className="rounded-full bg-[var(--color-surface-2)]/80 px-1.5 py-0.5 font-mono text-[10px] text-[var(--color-muted)] ring-1 ring-inset ring-[var(--color-border)]">
                          {item.badge}
                        </span>
                      )}
                    </button>
                  </li>
                )
              })}
            </ul>
          </div>
        ))}
      </nav>

      {/* Footer */}
      <div className="mt-auto px-3 pb-3 pt-2">
        <div className="mx-1 mb-2 h-px bg-[var(--color-border)]" />
        <div
          className={cn(
            'flex items-center gap-2 rounded-md p-2',
            collapsed ? 'justify-center' : 'bg-[var(--color-surface-2)]/40',
          )}
        >
          <div className="flex size-8 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-[var(--color-cta)] to-[var(--color-info)] font-mono text-xs font-semibold text-[var(--color-bg)]">
            OP
          </div>
          {!collapsed && (
            <div className="min-w-0 leading-tight">
              <div className="truncate text-xs font-medium">Ops engineer</div>
              <div className="truncate font-mono text-[10px] text-[var(--color-muted)]">
                on-call · plant A
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export function Sidebar() {
  const { collapsed, mobileOpen, setMobileOpen } = useSidebar()

  return (
    <>
      {/* Desktop floating sidebar */}
      <motion.aside
        aria-label="Primary navigation"
        animate={{ width: collapsed ? 64 : 240 }}
        transition={{ type: 'spring', stiffness: 320, damping: 32 }}
        className={cn(
          'fixed inset-y-2 left-2 z-30 hidden lg:block',
          'rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]/70',
          'backdrop-blur supports-[backdrop-filter]:bg-[var(--color-surface)]/55',
          'shadow-[0_0_0_1px_rgba(255,255,255,0.02),0_20px_50px_-20px_rgba(0,0,0,0.8)]',
          'overflow-hidden',
        )}
      >
        <SidebarBody />
      </motion.aside>

      {/* Mobile drawer */}
      <AnimatePresence>
        {mobileOpen && (
          <>
            <motion.div
              key="scrim"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm lg:hidden"
              onClick={() => setMobileOpen(false)}
              aria-hidden
            />
            <motion.aside
              key="drawer"
              initial={{ x: '-100%' }}
              animate={{ x: 0 }}
              exit={{ x: '-100%' }}
              transition={{ type: 'spring', stiffness: 320, damping: 32 }}
              className={cn(
                'fixed inset-y-2 left-2 z-50 w-[270px] lg:hidden',
                'rounded-xl border border-[var(--color-border)] bg-[var(--color-surface)]/95',
                'shadow-[0_0_0_1px_rgba(255,255,255,0.02),0_20px_50px_-20px_rgba(0,0,0,0.8)]',
                'overflow-hidden',
              )}
              role="dialog"
              aria-modal="true"
              aria-label="Primary navigation"
            >
              <div className="flex items-center justify-end p-2">
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="Close navigation"
                  onClick={() => setMobileOpen(false)}
                >
                  <X className="size-4" aria-hidden />
                </Button>
              </div>
              <SidebarBody onItemClick={() => setMobileOpen(false)} />
            </motion.aside>
          </>
        )}
      </AnimatePresence>
    </>
  )
}

/** Toggle button — collapses on desktop, opens drawer on mobile. */
export function SidebarTrigger({ className }: { className?: string }) {
  const { collapsed, setCollapsed, setMobileOpen } = useSidebar()
  return (
    <>
      <Button
        variant="ghost"
        size="icon"
        className={cn('hidden lg:inline-flex', className)}
        aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        aria-keyshortcuts="Control+B"
        onClick={() => setCollapsed(!collapsed)}
      >
        <PanelLeft className="size-4" aria-hidden />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className={cn('lg:hidden', className)}
        aria-label="Open navigation"
        onClick={() => setMobileOpen(true)}
      >
        <PanelLeft className="size-4" aria-hidden />
      </Button>
    </>
  )
}

/** Spacer/inset for the main content area so it doesn't sit under the floating sidebar.
 *  Mobile: no inset (sidebar is a drawer). Desktop: animated padding-left. */
export function SidebarInset({ children }: { children: React.ReactNode }) {
  const { collapsed } = useSidebar()
  const isDesktop =
    typeof window !== 'undefined' &&
    window.matchMedia('(min-width: 1024px)').matches

  const [desktop, setDesktop] = React.useState(isDesktop)
  React.useEffect(() => {
    const mq = window.matchMedia('(min-width: 1024px)')
    const onChange = () => setDesktop(mq.matches)
    mq.addEventListener('change', onChange)
    return () => mq.removeEventListener('change', onChange)
  }, [])

  return (
    <motion.div
      animate={{ paddingLeft: desktop ? (collapsed ? 80 : 256) : 0 }}
      transition={{ type: 'spring', stiffness: 320, damping: 32 }}
      className="min-h-screen"
    >
      {children}
    </motion.div>
  )
}
