import { motion } from 'motion/react'
import { Sparkles } from 'lucide-react'
import { Link, useRouterState } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import { NAV_GROUPS } from '@/lib/nav'
import { useLayout } from '@/context/layout-provider'
import { cn } from '@/lib/utils'
import { ActionDock } from '@/components/action-dock'
import { SidebarTrigger } from '@/components/ui/sidebar'

function Brand() {
  const intl = useIntl()
  return (
    <Link to="/" className="flex shrink-0 items-center gap-2.5 pl-2 pr-3">
      <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-[color:var(--color-leaf)] text-[color:var(--color-bg)]">
        <Sparkles className="h-4 w-4" strokeWidth={2.25} />
      </div>
      <span className="text-sm font-semibold tracking-tight text-[color:var(--color-text)]">
        {intl.formatMessage({ id: 'brand.name' })}
      </span>
      <span className="ml-1 hidden text-[10px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)] md:inline">
        {intl.formatMessage({ id: 'brand.tagline' })}
      </span>
    </Link>
  )
}

function HeaderNav() {
  const { location } = useRouterState()
  const intl = useIntl()
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
              isActive ? 'text-[color:var(--color-text)]' : 'text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]',
            )}
          >
            <Icon className="h-3.5 w-3.5" />
            <span className="whitespace-nowrap">{intl.formatMessage({ id: item.labelKey })}</span>
          </Comp>
        )
      })}
    </nav>
  )
}

export function TopHeader() {
  const { mode } = useLayout()
  const intl = useIntl()

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
                aria-label={intl.formatMessage({ id: 'a11y.openNav' })}
                className="text-[color:var(--color-muted)] hover:bg-[color:var(--color-surface-2)]/50 hover:text-[color:var(--color-text)] md:hidden"
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
      className="sticky top-0 z-30 flex h-16 items-center gap-3 border-b border-[color:var(--color-border)] bg-[color:var(--color-bg)]/80 px-4 backdrop-blur-md sm:px-6 lg:px-8"
    >
      <div className="ml-auto">
        <ActionDock inline />
      </div>
    </motion.header>
  )
}
