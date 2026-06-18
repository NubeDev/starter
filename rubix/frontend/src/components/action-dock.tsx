import { motion } from 'motion/react'
import {
  ChevronDown,
  Languages,
  Search,
} from 'lucide-react'
import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { useIntl } from 'react-intl'
import { useTheme, type Palette } from '@/stores/theme-store'
import { useLocale, LOCALES, type Locale } from '@/i18n'
import { cn } from '@/lib/utils'
import { ConfigDrawer } from '@/components/config-drawer'
import { ThemeSwitcher } from '@/components/theme-switcher'

const PALETTES: { id: Palette; labelKey: string; swatch: string }[] = [
  { id: 'nube',   labelKey: 'palette.nube',   swatch: 'linear-gradient(135deg,#339999,#184171)' },
  { id: 'ocean',  labelKey: 'palette.ocean',  swatch: 'linear-gradient(135deg,#3b82f6,#1e3a8a)' },
  { id: 'sunset', labelKey: 'palette.sunset', swatch: 'linear-gradient(135deg,#f97316,#b21368)' },
]

function SearchPill({ compact = false }: { compact?: boolean }) {
  const intl = useIntl()
  const label = intl.formatMessage({ id: 'a11y.searchButton' })
  // Compact: icon-only at every width. Used in the header-mode pill,
  // which is already nav-heavy and has no room for the wide search box.
  return (
    <>
      <button
        aria-label={label}
        className={cn(
          'flex h-9 w-9 cursor-pointer items-center justify-center rounded-full text-[color:var(--color-muted)] transition-colors hover:bg-[color:var(--color-surface-2)]/50 hover:text-[color:var(--color-text)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]',
          !compact && 'md:hidden',
        )}
      >
        <Search className="h-4 w-4" />
      </button>
      {!compact && (
        <button className="hidden h-9 w-56 cursor-pointer items-center gap-2 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 px-3.5 text-sm text-[color:var(--color-subtle)] transition-colors hover:border-[color:var(--color-border)] hover:bg-[color:var(--color-surface-2)]/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)] md:flex">
          <Search className="h-3.5 w-3.5 shrink-0" />
          <span className="flex-1 truncate text-left">{intl.formatMessage({ id: 'common.search' })}</span>
          <kbd className="shrink-0 rounded border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/50 px-1.5 py-[1px] font-sans text-[11px] leading-none tracking-wide">Ctrl K</kbd>
        </button>
      )}
    </>
  )
}

function ModeSwitcher() {
  return <ThemeSwitcher />
}

function PaletteMenu() {
  const { palette, setPalette } = useTheme()
  const intl = useIntl()
  const active = PALETTES.find((p) => p.id === palette)!
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          aria-label={intl.formatMessage({ id: 'a11y.paletteButton' })}
          title={intl.formatMessage({ id: 'a11y.paletteTitle' })}
          className="flex h-9 w-9 cursor-pointer items-center justify-center rounded-full transition-colors hover:bg-[color:var(--color-surface-2)]/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]"
        >
          <span
            className="h-4 w-4 rounded-full ring-1 ring-[color:var(--color-border)]"
            style={{ background: active.swatch }}
          />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="z-50 min-w-[10rem] rounded-xl border border-[color:var(--color-border)] bg-[color:var(--color-surface)] p-1 shadow-2xl"
        >
          {PALETTES.map((p) => (
            <DropdownMenu.Item
              key={p.id}
              onSelect={() => setPalette(p.id)}
              className="flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-sm text-[color:var(--color-muted)] outline-none transition-colors data-[highlighted]:bg-[color:var(--color-surface-2)]/60 data-[highlighted]:text-[color:var(--color-text)]"
            >
              <span
                className={cn(
                  'h-4 w-4 rounded-full ring-1 ring-[color:var(--color-border)]',
                  palette === p.id && 'ring-2 ring-[color:var(--color-leaf)]',
                )}
                style={{ background: p.swatch }}
              />
              <span className="flex-1">{intl.formatMessage({ id: p.labelKey })}</span>
              {palette === p.id && (
                <span className="text-[10px] uppercase tracking-wider text-[color:var(--color-leaf)]">
                  {intl.formatMessage({ id: 'common.active' })}
                </span>
              )}
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function LocaleMenu() {
  const locale = useLocale((s) => s.locale)
  const setLocale = useLocale((s) => s.setLocale)
  const intl = useIntl()
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          aria-label={intl.formatMessage({ id: 'a11y.localeButton' })}
          title={intl.formatMessage({ id: 'a11y.localeButton' })}
          className="flex h-9 w-9 cursor-pointer items-center justify-center rounded-full text-[color:var(--color-muted)] transition-colors hover:bg-[color:var(--color-surface-2)]/50 hover:text-[color:var(--color-text)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]"
        >
          <Languages className="h-4 w-4" />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="z-50 min-w-[8rem] rounded-xl border border-[color:var(--color-border)] bg-[color:var(--color-surface)] p-1 shadow-2xl"
        >
          {LOCALES.map((l) => (
            <DropdownMenu.Item
              key={l.id}
              onSelect={() => setLocale(l.id as Locale)}
              className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg px-2.5 py-1.5 text-sm text-[color:var(--color-muted)] outline-none transition-colors data-[highlighted]:bg-[color:var(--color-surface-2)]/60 data-[highlighted]:text-[color:var(--color-text)]"
            >
              <span>{l.label}</span>
              {locale === l.id && (
                <span className="text-[10px] uppercase tracking-wider text-[color:var(--color-leaf)]">
                  {intl.formatMessage({ id: 'common.active' })}
                </span>
              )}
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function UserMenu() {
  const intl = useIntl()
  const MENU = [
    { key: 'user.menu.profile' },
    { key: 'user.menu.tenants' },
    { key: 'user.menu.apiKeys' },
    { key: 'user.menu.billing' },
  ]
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button className="flex h-9 cursor-pointer items-center gap-2 rounded-full pl-1 pr-2.5 transition-colors hover:bg-[color:var(--color-surface-2)]/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]">
          <div className="grid h-7 w-7 place-items-center rounded-full bg-gradient-to-br from-[color:var(--color-leaf)] to-[color:var(--color-aqua)] text-[10px] font-semibold text-[color:var(--color-bg)]">
            AP
          </div>
          <ChevronDown className="h-3 w-3 text-[color:var(--color-subtle)]" />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="z-50 w-56 rounded-xl border border-[color:var(--color-border)] bg-[color:var(--color-surface)] p-1 text-sm shadow-2xl"
        >
          <div className="px-3 pb-1 pt-2">
            <div className="text-sm font-medium text-[color:var(--color-text)]">ap@nube-io.com</div>
            <div className="text-[11px] text-[color:var(--color-subtle)]">
              {intl.formatMessage({ id: 'user.role' })}
            </div>
          </div>
          <DropdownMenu.Separator className="my-1 h-px bg-[color:var(--color-surface-2)]/70" />
          {MENU.map((m) => (
            <DropdownMenu.Item
              key={m.key}
              className="cursor-pointer rounded-lg px-3 py-1.5 text-[color:var(--color-muted)] outline-none transition-colors data-[highlighted]:bg-[color:var(--color-surface-2)]/60 data-[highlighted]:text-[color:var(--color-text)]"
            >
              {intl.formatMessage({ id: m.key })}
            </DropdownMenu.Item>
          ))}
          <DropdownMenu.Separator className="my-1 h-px bg-[color:var(--color-surface-2)]/70" />
          <DropdownMenu.Item className="cursor-pointer rounded-lg px-3 py-1.5 text-[color:var(--color-danger)] outline-none transition-colors data-[highlighted]:bg-[color:var(--color-surface-2)]/60">
            {intl.formatMessage({ id: 'common.signOut' })}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

export function ActionDock({
  inline = false,
  includeTheme = true,
  includeUser = true,
  compactSearch = false,
}: {
  inline?: boolean
  /** Render the theme switcher. Set false where a theme control is
   * already provided alongside the dock (e.g. header-mode trailing,
   * which renders its own ThemeToggle) to avoid a duplicate. */
  includeTheme?: boolean
  /** Render the (placeholder) user menu. Set false where a real
   * auth-backed user menu is rendered alongside the dock. */
  includeUser?: boolean
  /** Use the icon-only search button instead of the wide search box.
   * Set in the space-constrained header-mode pill. */
  compactSearch?: boolean
} = {}) {
  if (inline) {
    return (
      <div className="flex items-center gap-1 sm:gap-2">
        <SearchPill compact={compactSearch} />
        {includeTheme && <ModeSwitcher />}
        <div className="hidden sm:contents">
          <LocaleMenu />
          <PaletteMenu />
          <ConfigDrawer />
        </div>
        {includeUser && <UserMenu />}
      </div>
    )
  }

  return (
    <motion.div
      initial={{ y: -60, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1], delay: 0.05 }}
      className="fixed right-4 top-3 z-40 sm:right-6 lg:right-8"
    >
      <div className="flex items-center gap-2">
        <SearchPill />
        <ModeSwitcher />
        <LocaleMenu />
        <PaletteMenu />
        <ConfigDrawer />
        <UserMenu />
      </div>
    </motion.div>
  )
}
