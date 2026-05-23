import { motion, AnimatePresence } from 'motion/react'
import {
  Sparkles,
  ArrowUpRight,
  Search,
  Settings,
  ChevronsLeft,
  ChevronsRight,
} from 'lucide-react'
import { Link, useRouterState } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { NAV_GROUPS } from '@/lib/nav'
import { cn } from '@/lib/utils'

interface FloatingSidebarProps {
  collapsed: boolean
  onToggleCollapse: () => void
}

export function FloatingSidebar({ collapsed, onToggleCollapse }: FloatingSidebarProps) {
  const { location } = useRouterState()
  const active = location.pathname

  return (
    <motion.aside
      initial={{ x: -40, opacity: 0 }}
      animate={{
        x: 0,
        opacity: 1,
        width: collapsed ? 76 : 264,
      }}
      exit={{ x: -40, opacity: 0 }}
      transition={{ duration: 0.6, ease: [0.22, 1, 0.36, 1] }}
      className="fixed left-4 top-4 bottom-4 z-50 hidden lg:block"
    >
      <div className="glass hairline relative flex h-full flex-col overflow-hidden rounded-3xl p-3">
        {/* Brand */}
        <div className="flex items-center gap-2.5 rounded-2xl px-2.5 py-2">
          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl bg-[color:var(--color-leaf)] text-[color:var(--color-bg)] shadow-[0_0_20px_rgba(51,153,153,0.4)]">
            <Sparkles className="h-4 w-4" strokeWidth={2.25} />
          </div>
          <AnimatePresence initial={false}>
            {!collapsed && (
              <motion.div
                key="brand-text"
                initial={{ opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -6 }}
                transition={{ duration: 0.2 }}
                className="min-w-0 flex-1"
              >
                <div className="truncate text-sm font-semibold tracking-tight">Nube</div>
                <div className="truncate text-[10px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
                  IoT Console
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>

        <div className="my-3 h-px w-full bg-white/[0.06]" />

        {/* Search */}
        {!collapsed && (
          <motion.button
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.1 }}
            className="mb-3 flex w-full items-center gap-2 rounded-xl border border-white/5 bg-white/[0.02] px-3 py-2 text-left text-xs text-[color:var(--color-subtle)] transition-colors hover:bg-white/[0.05] hover:text-white"
          >
            <Search className="h-3.5 w-3.5" />
            <span className="flex-1">Search devices, flows…</span>
            <kbd className="rounded-md border border-white/10 bg-white/[0.04] px-1.5 py-0.5 font-mono text-[10px]">
              ⌘K
            </kbd>
          </motion.button>
        )}

        {/* Nav */}
        <nav className="flex-1 space-y-5 overflow-y-auto">
          {NAV_GROUPS.map((group) => (
            <div key={group.title}>
              <AnimatePresence initial={false}>
                {!collapsed && (
                  <motion.div
                    key="group-title"
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: 'auto' }}
                    exit={{ opacity: 0, height: 0 }}
                    transition={{ duration: 0.2 }}
                    className="overflow-hidden px-3 pb-2 text-[10px] font-semibold uppercase tracking-[0.2em] text-[color:var(--color-subtle)]"
                  >
                    {group.title}
                  </motion.div>
                )}
              </AnimatePresence>
              <ul className="space-y-0.5">
                {group.items.map((item) => {
                  const isRoute = item.href.startsWith('/')
                  const isActive = isRoute ? active === item.href : false
                  const Icon = item.icon
                  const Comp: any = isRoute ? Link : 'a'
                  const linkProps: any = isRoute
                    ? { to: item.href }
                    : { href: item.href, onClick: (e: any) => e.preventDefault() }
                  return (
                    <li key={item.href}>
                      <Comp
                        {...linkProps}
                        className={cn(
                          'group relative flex items-center gap-3 rounded-xl px-3 py-2 text-sm transition-colors',
                          isActive
                            ? 'bg-[color:var(--color-leaf)]/10 text-white'
                            : 'text-[color:var(--color-muted)] hover:bg-white/[0.04] hover:text-white',
                        )}
                      >
                        {isActive && (
                          <motion.span
                            layoutId="sidebar-active"
                            className="absolute inset-0 rounded-xl ring-1 ring-[color:var(--color-leaf)]/30"
                            transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
                          />
                        )}
                        <Icon
                          className={cn(
                            'relative z-10 h-4 w-4 shrink-0',
                            isActive && 'text-[color:var(--color-leaf)]',
                          )}
                        />
                        <AnimatePresence initial={false}>
                          {!collapsed && (
                            <motion.span
                              key="label"
                              initial={{ opacity: 0, x: -4 }}
                              animate={{ opacity: 1, x: 0 }}
                              exit={{ opacity: 0, x: -4 }}
                              transition={{ duration: 0.2 }}
                              className="relative z-10 flex flex-1 items-center justify-between gap-2 whitespace-nowrap"
                            >
                              <span>{item.label}</span>
                              {item.badge && (
                                <span
                                  className={cn(
                                    'tabular rounded-full px-1.5 py-0.5 text-[10px] font-medium',
                                    isActive
                                      ? 'bg-[color:var(--color-leaf)]/20 text-[color:var(--color-leaf)]'
                                      : 'bg-white/[0.04] text-[color:var(--color-subtle)]',
                                  )}
                                >
                                  {item.badge}
                                </span>
                              )}
                            </motion.span>
                          )}
                        </AnimatePresence>
                      </Comp>
                    </li>
                  )
                })}
              </ul>
            </div>
          ))}
        </nav>

        <div className="my-3 h-px w-full bg-white/[0.06]" />

        {/* Footer */}
        <div className="space-y-2">
          {!collapsed ? (
            <Button size="sm" variant="leaf" className="w-full">
              New device
              <ArrowUpRight className="h-3.5 w-3.5" />
            </Button>
          ) : (
            <button className="flex h-9 w-full items-center justify-center rounded-xl bg-[color:var(--color-leaf)] text-[color:var(--color-bg)] transition-colors hover:bg-[color:var(--color-leaf-2)]">
              <ArrowUpRight className="h-4 w-4" />
            </button>
          )}
          <div className="flex items-center gap-1">
            <button
              onClick={onToggleCollapse}
              className="flex h-9 flex-1 items-center justify-center gap-2 rounded-xl border border-white/5 bg-white/[0.02] text-xs text-[color:var(--color-muted)] transition-colors hover:bg-white/[0.05] hover:text-white"
              aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            >
              {collapsed ? (
                <ChevronsRight className="h-4 w-4" />
              ) : (
                <>
                  <ChevronsLeft className="h-4 w-4" />
                  <span>Collapse</span>
                </>
              )}
            </button>
            {!collapsed && (
              <button className="flex h-9 w-9 items-center justify-center rounded-xl border border-white/5 bg-white/[0.02] text-[color:var(--color-muted)] transition-colors hover:bg-white/[0.05] hover:text-white">
                <Settings className="h-4 w-4" />
              </button>
            )}
          </div>
        </div>
      </div>
    </motion.aside>
  )
}
