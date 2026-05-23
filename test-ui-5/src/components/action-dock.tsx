import { motion } from 'motion/react'
import {
  ChevronDown,
  Monitor,
  Moon,
  Search,
  Sun,
} from 'lucide-react'
import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { useTheme, type Mode, type Palette } from '@/stores/theme-store'
import { cn } from '@/lib/utils'
import { ConfigDrawer } from '@/components/config-drawer'

const PALETTES: { id: Palette; label: string; swatch: string }[] = [
  { id: 'nube',   label: 'Nube',   swatch: 'linear-gradient(135deg,#339999,#184171)' },
  { id: 'ocean',  label: 'Ocean',  swatch: 'linear-gradient(135deg,#3b82f6,#1e3a8a)' },
  { id: 'sunset', label: 'Sunset', swatch: 'linear-gradient(135deg,#f97316,#b21368)' },
]

const MODE_ICON: Record<Mode, typeof Sun> = { light: Sun, dark: Moon, system: Monitor }

function SearchPill() {
  return (
    <>
      <button
        aria-label="Search"
        className="flex h-9 w-9 cursor-pointer items-center justify-center rounded-full text-[color:var(--color-muted)] transition-colors hover:bg-[color:var(--color-surface-2)]/50 hover:text-[color:var(--color-text)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)] md:hidden"
      >
        <Search className="h-4 w-4" />
      </button>
      <button className="hidden h-9 w-56 cursor-pointer items-center gap-2 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 px-3.5 text-sm text-[color:var(--color-subtle)] transition-colors hover:border-[color:var(--color-border)] hover:bg-[color:var(--color-surface-2)]/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)] md:flex">
        <Search className="h-3.5 w-3.5" />
        <span className="flex-1 truncate text-left">Search…</span>
        <kbd className="rounded border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/50 px-1.5 py-0.5 font-mono text-[10px]">⌘K</kbd>
      </button>
    </>
  )
}

function ModeSwitcher() {
  const { mode, setMode } = useTheme()
  const Icon = MODE_ICON[mode]
  const next: Mode = mode === 'light' ? 'dark' : mode === 'dark' ? 'system' : 'light'
  return (
    <button
      onClick={() => setMode(next)}
      aria-label={`Theme mode: ${mode}`}
      title={`Mode: ${mode} (click for ${next})`}
      className="flex h-9 w-9 cursor-pointer items-center justify-center rounded-full text-[color:var(--color-muted)] transition-colors hover:bg-[color:var(--color-surface-2)]/50 hover:text-[color:var(--color-text)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--color-ring)]"
    >
      <Icon className="h-4 w-4" />
    </button>
  )
}

function PaletteMenu() {
  const { palette, setPalette } = useTheme()
  const active = PALETTES.find((p) => p.id === palette)!
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          aria-label="Palette"
          title="Color palette"
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
              <span className="flex-1">{p.label}</span>
              {palette === p.id && (
                <span className="text-[10px] uppercase tracking-wider text-[color:var(--color-leaf)]">active</span>
              )}
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}


function UserMenu() {
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
            <div className="text-[11px] text-[color:var(--color-subtle)]">Admin · Acme Energy</div>
          </div>
          <DropdownMenu.Separator className="my-1 h-px bg-[color:var(--color-surface-2)]/70" />
          {['Profile', 'Tenants', 'API keys', 'Billing'].map((l) => (
            <DropdownMenu.Item
              key={l}
              className="cursor-pointer rounded-lg px-3 py-1.5 text-[color:var(--color-muted)] outline-none transition-colors data-[highlighted]:bg-[color:var(--color-surface-2)]/60 data-[highlighted]:text-[color:var(--color-text)]"
            >
              {l}
            </DropdownMenu.Item>
          ))}
          <DropdownMenu.Separator className="my-1 h-px bg-[color:var(--color-surface-2)]/70" />
          <DropdownMenu.Item className="cursor-pointer rounded-lg px-3 py-1.5 text-[color:var(--color-danger)] outline-none transition-colors data-[highlighted]:bg-[color:var(--color-surface-2)]/60">
            Sign out
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

export function ActionDock({ inline = false }: { inline?: boolean } = {}) {
  if (inline) {
    return (
      <div className="flex items-center gap-1 sm:gap-2">
        <SearchPill />
        <ModeSwitcher />
        <div className="hidden sm:contents">
          <PaletteMenu />
          <ConfigDrawer />
        </div>
        <UserMenu />
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
        <PaletteMenu />
        <ConfigDrawer />
        <UserMenu />
      </div>
    </motion.div>
  )
}
