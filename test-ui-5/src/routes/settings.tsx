import { createFileRoute } from '@tanstack/react-router'
import { motion } from 'motion/react'
import { useTheme, type Mode, type Palette } from '@/stores/theme-store'
import { LayoutToggle } from '@/components/layout-toggle'
import { cn } from '@/lib/utils'

const MODES: { id: Mode; label: string }[] = [
  { id: 'light',  label: 'Light' },
  { id: 'dark',   label: 'Dark' },
  { id: 'system', label: 'System' },
]

const PALETTES: { id: Palette; label: string; swatch: string }[] = [
  { id: 'nube',   label: 'Nube',   swatch: 'linear-gradient(135deg,#339999,#184171)' },
  { id: 'ocean',  label: 'Ocean',  swatch: 'linear-gradient(135deg,#3b82f6,#1e3a8a)' },
  { id: 'sunset', label: 'Sunset', swatch: 'linear-gradient(135deg,#f97316,#b21368)' },
]

function Row({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-1 gap-4 border-b border-white/[0.04] py-6 last:border-b-0 md:grid-cols-[260px_1fr]">
      <div>
        <div className="text-sm font-medium text-white">{title}</div>
        {hint && <div className="mt-1 text-xs text-[color:var(--color-subtle)]">{hint}</div>}
      </div>
      <div className="flex items-center">{children}</div>
    </div>
  )
}

function Settings() {
  const { mode, palette, setMode, setPalette } = useTheme()

  return (
    <section className="relative mx-auto max-w-4xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <motion.div
        initial={{ opacity: 0, y: 14 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6, ease: [0.22, 1, 0.36, 1] }}
        className="mb-10"
      >
        <div className="flex items-center gap-3">
          <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
            Settings
          </span>
        </div>
        <h1 className="mt-3 text-4xl font-medium leading-[1.05] tracking-[-0.03em] text-white sm:text-5xl">
          Appearance
        </h1>
        <p className="mt-2 text-sm text-[color:var(--color-muted)]">
          Make the console feel like home. Changes apply instantly and persist across sessions.
        </p>
      </motion.div>

      <div className="glass rounded-3xl p-6 sm:p-8">
        <Row title="Layout" hint="Where the primary navigation lives.">
          <LayoutToggle />
        </Row>

        <Row title="Mode" hint="Light, dark, or follow your OS.">
          <div className="inline-flex items-center gap-1 rounded-full border border-white/[0.06] bg-white/[0.02] p-1">
            {MODES.map((m) => (
              <button
                key={m.id}
                onClick={() => setMode(m.id)}
                className={cn(
                  'cursor-pointer rounded-full px-3 py-1.5 text-xs font-medium transition-colors',
                  mode === m.id
                    ? 'bg-[color:var(--color-leaf)] text-[color:var(--color-bg)]'
                    : 'text-[color:var(--color-muted)] hover:text-white',
                )}
              >
                {m.label}
              </button>
            ))}
          </div>
        </Row>

        <Row title="Palette" hint="Brand accent. Swap any time.">
          <div className="flex items-center gap-3">
            {PALETTES.map((p) => {
              const active = palette === p.id
              return (
                <button
                  key={p.id}
                  onClick={() => setPalette(p.id)}
                  className={cn(
                    'group flex cursor-pointer flex-col items-center gap-1.5 transition-transform hover:-translate-y-0.5',
                  )}
                >
                  <span
                    className={cn(
                      'h-10 w-10 rounded-full ring-1 ring-white/20 ring-offset-2 ring-offset-[color:var(--color-bg)] transition-all',
                      active && 'ring-2 ring-[color:var(--color-leaf)]',
                    )}
                    style={{ background: p.swatch }}
                  />
                  <span className={cn('text-[11px]', active ? 'text-white' : 'text-[color:var(--color-muted)]')}>
                    {p.label}
                  </span>
                </button>
              )
            })}
          </div>
        </Row>

        <Row title="Density" hint="Coming soon — comfortable, compact.">
          <span className="text-xs text-[color:var(--color-subtle)]">Default</span>
        </Row>

        <Row title="Motion" hint="Honors prefers-reduced-motion automatically.">
          <span className="text-xs text-[color:var(--color-subtle)]">Auto</span>
        </Row>
      </div>
    </section>
  )
}

export const Route = createFileRoute('/settings')({ component: Settings })
