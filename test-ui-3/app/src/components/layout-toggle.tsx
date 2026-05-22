import { motion } from 'motion/react'
import { PanelLeft, LayoutPanelTop } from 'lucide-react'
import { useLayout } from '@/context/layout-provider'
import { cn } from '@/lib/utils'

export function LayoutToggle() {
  const { mode, setMode } = useLayout()
  return (
    <motion.div
      initial={{ opacity: 0, y: -10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.6, delay: 3, ease: [0.22, 1, 0.36, 1] }}
      className="glass fixed right-4 top-4 z-[60] hidden items-center gap-1 rounded-full p-1 lg:flex"
    >
      <button
        onClick={() => setMode('header')}
        className={cn(
          'relative flex items-center gap-1.5 rounded-full px-3 py-1.5 text-[11px] font-medium transition-colors',
          mode === 'header' ? 'text-[color:var(--color-bg)]' : 'text-[color:var(--color-muted)] hover:text-white',
        )}
        aria-pressed={mode === 'header'}
        aria-label="Use header layout"
      >
        {mode === 'header' && (
          <motion.span
            layoutId="layout-toggle-pill"
            className="absolute inset-0 rounded-full bg-[color:var(--color-leaf)]"
            transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
          />
        )}
        <LayoutPanelTop className="relative z-10 h-3.5 w-3.5" />
        <span className="relative z-10">Header</span>
      </button>
      <button
        onClick={() => setMode('sidebar')}
        className={cn(
          'relative flex items-center gap-1.5 rounded-full px-3 py-1.5 text-[11px] font-medium transition-colors',
          mode === 'sidebar' ? 'text-[color:var(--color-bg)]' : 'text-[color:var(--color-muted)] hover:text-white',
        )}
        aria-pressed={mode === 'sidebar'}
        aria-label="Use sidebar layout"
      >
        {mode === 'sidebar' && (
          <motion.span
            layoutId="layout-toggle-pill"
            className="absolute inset-0 rounded-full bg-[color:var(--color-leaf)]"
            transition={{ duration: 0.45, ease: [0.22, 1, 0.36, 1] }}
          />
        )}
        <PanelLeft className="relative z-10 h-3.5 w-3.5" />
        <span className="relative z-10">Sidebar</span>
      </button>
    </motion.div>
  )
}
