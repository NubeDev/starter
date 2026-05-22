import { motion } from 'motion/react'
import { PanelLeft, LayoutPanelTop, Sparkles, Square } from 'lucide-react'
import { useLayout } from '@/context/layout-provider'
import { useHeroVariant } from '@/context/hero-variant-provider'
import { cn } from '@/lib/utils'

function Segmented<T extends string>({
  value,
  onChange,
  options,
  layoutId,
}: {
  value: T
  onChange: (v: T) => void
  options: Array<{ value: T; label: string; icon: React.ComponentType<{ className?: string }> }>
  layoutId: string
}) {
  return (
    <div className="glass flex items-center gap-1 rounded-full p-1">
      {options.map((opt) => {
        const Icon = opt.icon
        const active = value === opt.value
        return (
          <button
            key={opt.value}
            onClick={() => onChange(opt.value)}
            className={cn(
              'relative flex items-center gap-1.5 rounded-full px-3 py-1.5 text-[11px] font-medium transition-colors',
              active
                ? 'text-[color:var(--color-bg)]'
                : 'text-[color:var(--color-muted)] hover:text-white',
            )}
            aria-pressed={active}
            aria-label={opt.label}
          >
            {active && (
              <motion.span
                layoutId={layoutId}
                className="absolute inset-0 rounded-full bg-[color:var(--color-leaf)]"
                transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
              />
            )}
            <Icon className="relative z-10 h-3.5 w-3.5" />
            <span className="relative z-10">{opt.label}</span>
          </button>
        )
      })}
    </div>
  )
}

export function LayoutToggle() {
  const { mode, setMode } = useLayout()
  const { variant, setVariant } = useHeroVariant()

  return (
    <motion.div
      initial={{ opacity: 0, y: -10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.6, delay: 3, ease: [0.22, 1, 0.36, 1] }}
      className="fixed right-4 top-4 z-[60] hidden flex-col items-end gap-2 lg:flex"
    >
      <Segmented
        value={mode}
        onChange={setMode}
        layoutId="layout-toggle-pill"
        options={[
          { value: 'header', label: 'Header', icon: LayoutPanelTop },
          { value: 'sidebar', label: 'Sidebar', icon: PanelLeft },
        ]}
      />
      <Segmented
        value={variant}
        onChange={setVariant}
        layoutId="hero-toggle-pill"
        options={[
          { value: 'glass', label: 'Glass', icon: Square },
          { value: 'shader', label: 'Shader', icon: Sparkles },
        ]}
      />
    </motion.div>
  )
}
