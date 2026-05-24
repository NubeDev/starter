import { motion } from 'motion/react'
import { LayoutPanelTop, PanelLeft } from 'lucide-react'
import { useIntl } from 'react-intl'
import { useLayout } from '@/context/layout-provider'
import { cn } from '@/lib/utils'

/**
 * Inline segmented control. Lives inside Settings → Appearance now,
 * not as a floating overlay.
 */
export function LayoutToggle() {
  const { mode, setMode } = useLayout()
  const intl = useIntl()
  return (
    <div className="inline-flex items-center gap-1 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)]/30 p-1">
      <button
        onClick={() => setMode('header')}
        className={cn(
          'relative flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-medium transition-colors',
          mode === 'header' ? 'text-[color:var(--color-bg)]' : 'text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]',
        )}
        aria-pressed={mode === 'header'}
      >
        {mode === 'header' && (
          <motion.span
            layoutId="layout-toggle-pill"
            className="absolute inset-0 rounded-full bg-[color:var(--color-leaf)]"
            transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
          />
        )}
        <LayoutPanelTop className="relative z-10 h-3.5 w-3.5" />
        <span className="relative z-10">{intl.formatMessage({ id: 'layoutToggle.header' })}</span>
      </button>
      <button
        onClick={() => setMode('sidebar')}
        className={cn(
          'relative flex cursor-pointer items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-medium transition-colors',
          mode === 'sidebar' ? 'text-[color:var(--color-bg)]' : 'text-[color:var(--color-muted)] hover:text-[color:var(--color-text)]',
        )}
        aria-pressed={mode === 'sidebar'}
      >
        {mode === 'sidebar' && (
          <motion.span
            layoutId="layout-toggle-pill"
            className="absolute inset-0 rounded-full bg-[color:var(--color-leaf)]"
            transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
          />
        )}
        <PanelLeft className="relative z-10 h-3.5 w-3.5" />
        <span className="relative z-10">{intl.formatMessage({ id: 'layoutToggle.sidebar' })}</span>
      </button>
    </div>
  )
}
