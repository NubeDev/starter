import { motion } from 'motion/react'
import { Sparkles } from 'lucide-react'
import { Link, useRouterState } from '@tanstack/react-router'
import { NAV_GROUPS } from '@/lib/nav'
import { useLayout } from '@/context/layout-provider'
import { cn } from '@/lib/utils'
import { ActionDock } from '@/components/action-dock'
import { SidebarTrigger } from '@/components/ui/sidebar'

function Brand() {
  return (
    <Link to="/" className="flex shrink-0 items-center gap-2.5 pl-2 pr-3">
      <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-[color:var(--color-leaf)] text-[color:var(--color-bg)]">
        <Sparkles className="h-4 w-4" strokeWidth={2.25} />
      </div>
      <span className="text-sm font-semibold tracking-tight text-white">Nube</span>
      <span className="ml-1 hidden text-[10px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)] md:inline">
        IoT Console
      </span>
    </Link>
  )
}

function HeaderNav() {
  const { location } = useRouterState()
  const items = NAV_GROUPS.flatMap((g) => g.items).slice(0, 5)
  return (
    <nav className="hidden items-center gap-5 text-sm md:flex">
      {items.map((item) => {
        const isRoute = item.href.startsWith('/')
        const isActive = isRoute && location.pathname === item.href
        const Icon = item.icon
        const Comp: any = isRoute ? Link : 'a'
        const linkProps: any = isRoute
          ? { to: item.href }
          : { href: item.href, onClick: (e: any) => e.preventDefault() }
        return (
          <Comp
            key={item.href}
            {...linkProps}
            className={cn(
              'flex cursor-pointer items-center gap-1.5 transition-colors',
              isActive ? 'text-white' : 'text-[color:var(--color-muted)] hover:text-white',
            )}
          >
            <Icon className="h-3.5 w-3.5" />
            <span className="whitespace-nowrap">{item.label}</span>
          </Comp>
        )
      })}
    </nav>
  )
}

export function TopHeader() {
  const { mode } = useLayout()

  if (mode === 'header') {
    return (
      <motion.header
        initial={{ y: -60, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
        className="fixed inset-x-0 top-3 z-40"
      >
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="glass flex items-center justify-between gap-3 rounded-full py-2 pl-2 pr-2 sm:gap-6 sm:pl-3 sm:pr-3">
            <div className="flex items-center gap-2 sm:gap-6">
              <SidebarTrigger
                data-testid="mobile-nav-trigger"
                aria-label="Open navigation menu"
                className="text-[color:var(--color-muted)] hover:bg-white/[0.04] hover:text-white md:hidden"
              />
              <Brand />
              <HeaderNav />
            </div>
            <ActionDock inline />
          </div>
        </div>
      </motion.header>
    )
  }

  // Sidebar mode: header bar lives inside the content column (right of sidebar),
  // matching shadcn admin's floating-sidebar layout.
  return (
    <motion.header
      initial={{ y: -16, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
      className="sticky top-0 z-30 flex h-16 items-center gap-3 border-b border-white/[0.06] bg-[color:var(--color-bg)]/80 px-4 backdrop-blur-md sm:px-6 lg:px-8"
    >
      <div className="ml-auto">
        <ActionDock inline />
      </div>
    </motion.header>
  )
}
