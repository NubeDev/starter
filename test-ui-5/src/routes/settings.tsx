import { createFileRoute } from '@tanstack/react-router'
import { motion } from 'motion/react'
import { useIntl } from 'react-intl'
import { useTheme, FONT_STACKS, type Mode, type Palette, type Font } from '@/stores/theme-store'
import { useLayout, type Variant, type Collapsible } from '@/context/layout-provider'
import { LayoutToggle } from '@/components/layout-toggle'
import { cn } from '@/lib/utils'

const MODES: { id: Mode; labelKey: string }[] = [
  { id: 'light',  labelKey: 'mode.light' },
  { id: 'dark',   labelKey: 'mode.dark' },
  { id: 'system', labelKey: 'mode.system' },
]

const PALETTES: { id: Palette; labelKey: string; swatch: string }[] = [
  { id: 'nube',   labelKey: 'palette.nube',   swatch: 'linear-gradient(135deg,#339999,#184171)' },
  { id: 'ocean',  labelKey: 'palette.ocean',  swatch: 'linear-gradient(135deg,#3b82f6,#1e3a8a)' },
  { id: 'sunset', labelKey: 'palette.sunset', swatch: 'linear-gradient(135deg,#f97316,#b21368)' },
]

const FONTS: { id: Font; labelKey: string }[] = [
  { id: 'geist',   labelKey: 'font.geist' },
  { id: 'inter',   labelKey: 'font.inter' },
  { id: 'manrope', labelKey: 'font.manrope' },
  { id: 'system',  labelKey: 'font.system' },
]

const VARIANTS: { id: Variant; labelKey: string; hintKey: string }[] = [
  { id: 'inset',    labelKey: 'settings.variant.inset',    hintKey: 'settings.variant.inset.hint' },
  { id: 'sidebar',  labelKey: 'settings.variant.sidebar',  hintKey: 'settings.variant.sidebar.hint' },
  { id: 'floating', labelKey: 'settings.variant.floating', hintKey: 'settings.variant.floating.hint' },
]

const COLLAPSIBLES: { id: Collapsible; labelKey: string; hintKey: string }[] = [
  { id: 'icon',      labelKey: 'settings.collapsible.icon',      hintKey: 'settings.collapsible.icon.hint' },
  { id: 'offcanvas', labelKey: 'settings.collapsible.offcanvas', hintKey: 'settings.collapsible.offcanvas.hint' },
  { id: 'none',      labelKey: 'settings.collapsible.none',      hintKey: 'settings.collapsible.none.hint' },
]

function Row({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-1 gap-4 border-b border-[color:var(--color-border)] py-6 last:border-b-0 md:grid-cols-[260px_1fr]">
      <div>
        <div className="text-sm font-medium text-[color:var(--color-text)]">{title}</div>
        {hint && <div className="mt-1 text-xs text-[color:var(--color-subtle)]">{hint}</div>}
      </div>
      <div className="flex items-center">{children}</div>
    </div>
  )
}

function Settings() {
  const { mode, palette, font, setMode, setPalette, setFont } = useTheme()
  const { mode: shellMode, variant, setVariant, collapsible, setCollapsible } = useLayout()
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })

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
            {tr('settings.eyebrow')}
          </span>
        </div>
        <h1 className="mt-3 text-4xl font-medium leading-[1.05] tracking-[-0.03em] text-[color:var(--color-text)] sm:text-5xl">
          {tr('settings.title')}
        </h1>
        <p className="mt-2 text-sm text-[color:var(--color-muted)]">
          {tr('settings.subtitle')}
        </p>
      </motion.div>

      <div className="glass rounded-3xl p-6 sm:p-8">
        <Row title={tr('settings.row.layout')} hint={tr('settings.row.layout.hint')}>
          <LayoutToggle />
        </Row>

        {shellMode === 'sidebar' && (
          <>
            <Row title={tr('settings.row.sidebarVariant')} hint={tr('settings.row.sidebarVariant.hint')}>
              <div className="inline-flex flex-wrap items-center gap-1 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 p-1">
                {VARIANTS.map((v) => (
                  <button
                    key={v.id}
                    onClick={() => setVariant(v.id)}
                    title={tr(v.hintKey)}
                    className={cn(
                      'cursor-pointer rounded-full px-3 py-1.5 text-xs font-medium transition-colors',
                      variant === v.id
                        ? 'bg-[color:var(--color-leaf)] text-[color:var(--color-bg)]'
                        : 'text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]',
                    )}
                  >
                    {tr(v.labelKey)}
                  </button>
                ))}
              </div>
            </Row>

            <Row title={tr('settings.row.sidebarCollapse')} hint={tr('settings.row.sidebarCollapse.hint')}>
              <div className="inline-flex flex-wrap items-center gap-1 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 p-1">
                {COLLAPSIBLES.map((c) => (
                  <button
                    key={c.id}
                    onClick={() => setCollapsible(c.id)}
                    title={tr(c.hintKey)}
                    className={cn(
                      'cursor-pointer rounded-full px-3 py-1.5 text-xs font-medium transition-colors',
                      collapsible === c.id
                        ? 'bg-[color:var(--color-leaf)] text-[color:var(--color-bg)]'
                        : 'text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]',
                    )}
                  >
                    {tr(c.labelKey)}
                  </button>
                ))}
              </div>
            </Row>
          </>
        )}

        <Row title={tr('settings.row.mode')} hint={tr('settings.row.mode.hint')}>
          <div className="inline-flex items-center gap-1 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 p-1">
            {MODES.map((m) => (
              <button
                key={m.id}
                onClick={() => setMode(m.id)}
                className={cn(
                  'cursor-pointer rounded-full px-3 py-1.5 text-xs font-medium transition-colors',
                  mode === m.id
                    ? 'bg-[color:var(--color-leaf)] text-[color:var(--color-bg)]'
                    : 'text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]',
                )}
              >
                {tr(m.labelKey)}
              </button>
            ))}
          </div>
        </Row>

        <Row title={tr('settings.row.palette')} hint={tr('settings.row.palette.hint')}>
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
                      'h-10 w-10 rounded-full ring-1 ring-[color:var(--color-border)] ring-offset-2 ring-offset-[color:var(--color-bg)] transition-all',
                      active && 'ring-2 ring-[color:var(--color-leaf)]',
                    )}
                    style={{ background: p.swatch }}
                  />
                  <span className={cn('text-[11px]', active ? 'text-[color:var(--color-text)]' : 'text-[color:var(--color-muted)]')}>
                    {tr(p.labelKey)}
                  </span>
                </button>
              )
            })}
          </div>
        </Row>

        <Row title={tr('settings.row.font')} hint={tr('settings.row.font.hint')}>
          <div className="inline-flex flex-wrap items-center gap-1 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 p-1">
            {FONTS.map((f) => (
              <button
                key={f.id}
                onClick={() => setFont(f.id)}
                style={{ fontFamily: FONT_STACKS[f.id] }}
                className={cn(
                  'cursor-pointer rounded-full px-3 py-1.5 text-xs font-medium transition-colors',
                  font === f.id
                    ? 'bg-[color:var(--color-leaf)] text-[color:var(--color-bg)]'
                    : 'text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]',
                )}
              >
                {tr(f.labelKey)}
              </button>
            ))}
          </div>
        </Row>

        <Row title={tr('settings.row.density')} hint={tr('settings.row.density.hint')}>
          <span className="text-xs text-[color:var(--color-subtle)]">{tr('common.default')}</span>
        </Row>

        <Row title={tr('settings.row.motion')} hint={tr('settings.row.motion.hint')}>
          <span className="text-xs text-[color:var(--color-subtle)]">{tr('common.auto')}</span>
        </Row>
      </div>
    </section>
  )
}

export const Route = createFileRoute('/settings')({ component: Settings })
