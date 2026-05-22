import { AnimatePresence, motion } from 'motion/react'
import { AlertTriangle, Info, ShieldAlert } from 'lucide-react'
import { Badge } from '@/components/ui/badge'

export type Alert = {
  id: string
  level: 'info' | 'warn' | 'danger'
  device: string
  message: string
  at: string
}

const ICON = {
  info: <Info className="size-4" aria-hidden />,
  warn: <AlertTriangle className="size-4" aria-hidden />,
  danger: <ShieldAlert className="size-4" aria-hidden />,
}

const TONE = { info: 'info', warn: 'warn', danger: 'danger' } as const

export function AlertFeed({ alerts }: { alerts: Alert[] }) {
  return (
    <ul className="space-y-2">
      <AnimatePresence initial={false}>
        {alerts.map((a) => (
          <motion.li
            key={a.id}
            layout
            initial={{ opacity: 0, y: -6, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, x: 20 }}
            transition={{ duration: 0.2, ease: 'easeOut' }}
            className="flex items-start gap-3 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface-2)]/30 p-3"
          >
            <span
              className={
                a.level === 'danger'
                  ? 'text-[var(--color-danger)]'
                  : a.level === 'warn'
                    ? 'text-[var(--color-warn)]'
                    : 'text-[var(--color-info)]'
              }
            >
              {ICON[a.level]}
            </span>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <Badge tone={TONE[a.level]}>{a.level}</Badge>
                <span className="font-mono text-xs text-[var(--color-muted)]">{a.device}</span>
                <span className="ml-auto font-mono text-xs text-[var(--color-muted)]">{a.at}</span>
              </div>
              <p className="mt-1 text-sm">{a.message}</p>
            </div>
          </motion.li>
        ))}
      </AnimatePresence>
    </ul>
  )
}
