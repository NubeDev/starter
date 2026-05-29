import { motion } from 'motion/react'
import { Sparkles, LogOut, ChevronDown, Building2 } from 'lucide-react'
import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { Link, useRouterState } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import { useAuth } from '@nube/starter-client-react'
import { useTenants } from '@nube/starter-ui-authz'
import { NAV_GROUPS } from '@/lib/nav'
import { useLayout } from '@nube/starter-ui-core/layout'
import { useTheme } from '@/stores/theme-store'
import { cn } from '@/lib/utils'
import { ActionDock } from '@/components/action-dock'
import { ThemeSwitcher } from '@/components/theme-switcher'
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

/**
 * Theme switcher: three-segment pill (system / light / dark) inspired
 * by kibo-ui's ThemeSwitcher, wired to `useTheme().setMode`.
 */
function ThemeToggle() {
  return <ThemeSwitcher />
}

/**
 * Tenant indicator: display-only when the operator belongs to 0 or 1
 * tenants; a disabled dropdown listing all tenants when there are
 * 2+. Switching is not yet wired — the items render with a tooltip
 * explaining that.
 */
function TenantIndicator() {
  const intl = useIntl()
  const tenantsQuery = useTenants()
  const tenants = tenantsQuery.data ?? []

  if (tenants.length === 0) {
    return (
      <span
        title={intl.formatMessage({ id: 'header.tenant.label' })}
        className="hidden items-center gap-1.5 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 px-2.5 py-1 text-xs text-[color:var(--color-subtle)] md:inline-flex"
      >
        <Building2 className="h-3.5 w-3.5" />
        {intl.formatMessage({ id: 'header.tenant.none' })}
      </span>
    )
  }

  if (tenants.length === 1) {
    const t = tenants[0]
    return (
      <span
        title={intl.formatMessage({ id: 'header.tenant.label' })}
        className="hidden items-center gap-1.5 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 px-2.5 py-1 text-xs text-[color:var(--color-text)] md:inline-flex"
      >
        <Building2 className="h-3.5 w-3.5" />
        <span data-testid="tenant-indicator-name">{t.display_name}</span>
      </span>
    )
  }

  // 2+ tenants: disabled dropdown with a tooltip on each item.
  const active = tenants[0]
  const notWired = intl.formatMessage({ id: 'header.tenant.switchingNotWired' })
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          type="button"
          title={notWired}
          aria-label={intl.formatMessage({ id: 'header.tenant.label' })}
          className="hidden cursor-pointer items-center gap-1.5 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 px-2.5 py-1 text-xs text-[color:var(--color-text)] transition-colors hover:bg-[color:var(--color-surface-2)]/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)] md:inline-flex"
        >
          <Building2 className="h-3.5 w-3.5" />
          <span>{active.display_name}</span>
          <ChevronDown className="h-3 w-3 opacity-60" />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="z-50 min-w-[12rem] rounded-xl border border-[color:var(--color-border)] bg-[color:var(--color-surface)] p-1 shadow-2xl"
        >
          <div className="px-2.5 py-1 text-[10px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {notWired}
          </div>
          {tenants.map((t) => (
            <DropdownMenu.Item
              key={t.id}
              disabled
              title={notWired}
              className="flex cursor-not-allowed items-center gap-2 rounded-lg px-2.5 py-1.5 text-sm text-[color:var(--color-muted)] outline-none opacity-60"
            >
              <Building2 className="h-3.5 w-3.5" />
              <span className="flex-1 truncate">{t.display_name}</span>
              <span className="ml-2 text-[10px] uppercase tracking-wider text-[color:var(--color-subtle)]">
                {t.slug}
              </span>
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

const ROLE_BADGE_CLASS: Record<string, string> = {
  admin: 'bg-[color:var(--color-leaf)]/15 text-[color:var(--color-leaf)] border-[color:var(--color-leaf)]/30',
  writer: 'bg-[color:var(--color-surface-2)]/60 text-[color:var(--color-text)] border-[color:var(--color-border)]',
  reader: 'bg-[color:var(--color-surface-2)]/40 text-[color:var(--color-subtle)] border-[color:var(--color-border)]',
}

function UserMenu() {
  const intl = useIntl()
  const { user, logout } = useAuth()
  if (!user) return null
  const role = user.role
  const badgeClass = ROLE_BADGE_CLASS[role] ?? ROLE_BADGE_CLASS.reader
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          type="button"
          aria-label={intl.formatMessage({ id: 'header.userMenu' })}
          className="flex h-9 cursor-pointer items-center gap-2 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 px-2.5 text-xs transition-colors hover:bg-[color:var(--color-surface-2)]/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]"
        >
          <span
            data-testid="user-email"
            className="hidden max-w-[14rem] truncate text-[color:var(--color-text)] md:inline"
          >
            {user.email}
          </span>
          <span
            data-testid="user-role-badge"
            className={cn(
              'inline-flex items-center rounded-full border px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wider',
              badgeClass,
            )}
          >
            {intl.formatMessage({ id: `header.role.${role}` })}
          </span>
          <ChevronDown className="h-3 w-3 text-[color:var(--color-muted)]" />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="z-50 min-w-[14rem] rounded-xl border border-[color:var(--color-border)] bg-[color:var(--color-surface)] p-1 shadow-2xl"
        >
          <div className="px-2.5 py-2 text-xs text-[color:var(--color-subtle)]">
            <div className="truncate text-[color:var(--color-text)]">{user.email}</div>
            <div className="mt-0.5">{intl.formatMessage({ id: `header.role.${role}` })}</div>
          </div>
          <DropdownMenu.Separator className="my-1 h-px bg-[color:var(--color-border)]" />
          <DropdownMenu.Item
            data-testid="logout-menu-item"
            onSelect={() => {
              void logout()
            }}
            className="flex cursor-pointer items-center gap-2 rounded-lg px-2.5 py-1.5 text-sm text-[color:var(--color-muted)] outline-none transition-colors data-[highlighted]:bg-[color:var(--color-surface-2)]/60 data-[highlighted]:text-[color:var(--color-text)]"
          >
            <LogOut className="h-3.5 w-3.5" />
            <span>{intl.formatMessage({ id: 'header.logout' })}</span>
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function HeaderTrailing() {
  return (
    <div className="flex items-center gap-2">
      <TenantIndicator />
      <ThemeToggle />
      <UserMenu />
      <ActionDock inline />
    </div>
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
                variant='outline'
                className="size-8 border-[color:var(--color-border)] bg-[color:var(--color-surface)] text-[color:var(--color-text)] hover:bg-[color:var(--color-surface-2)]/60 hover:text-[color:var(--color-text)] md:hidden"
              />
              <Brand />
              <HeaderNav />
            </div>
            <HeaderTrailing />
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
        <HeaderTrailing />
      </div>
    </motion.header>
  )
}
