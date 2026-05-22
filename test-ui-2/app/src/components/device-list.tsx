import { AnimatePresence, motion } from 'motion/react'
import { Cpu, Wifi, WifiOff } from 'lucide-react'
import { Badge, StatusDot } from '@/components/ui/badge'

export type Device = {
  id: string
  name: string
  location: string
  status: 'online' | 'degraded' | 'offline'
  load: number
  battery: number
}

const STATUS_TONE = {
  online: 'ok',
  degraded: 'warn',
  offline: 'danger',
} as const

export function DeviceList({ devices }: { devices: Device[] }) {
  return (
    <ul className="divide-y divide-[var(--color-border)]">
      <AnimatePresence initial={false}>
        {devices.map((d) => (
          <motion.li
            key={d.id}
            layout
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 8 }}
            transition={{ duration: 0.2 }}
            className="flex items-center gap-3 px-4 py-3"
          >
            <div className="flex size-9 items-center justify-center rounded-lg bg-[var(--color-surface-2)]/60 text-[var(--color-info)]">
              <Cpu className="size-4" aria-hidden />
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="truncate text-sm font-medium">{d.name}</span>
                <Badge tone={STATUS_TONE[d.status]}>
                  <StatusDot tone={STATUS_TONE[d.status]} />
                  {d.status}
                </Badge>
              </div>
              <div className="font-mono text-xs text-[var(--color-muted)]">
                {d.id} · {d.location}
              </div>
            </div>
            <div className="hidden text-right sm:block">
              <div className="font-mono text-xs text-[var(--color-muted)]">load</div>
              <div className="font-mono text-sm tabular-nums">{d.load.toFixed(0)}%</div>
            </div>
            <div className="hidden text-right sm:block">
              <div className="font-mono text-xs text-[var(--color-muted)]">batt</div>
              <div className="font-mono text-sm tabular-nums">{d.battery.toFixed(0)}%</div>
            </div>
            <div className="ml-1">
              {d.status === 'offline' ? (
                <WifiOff className="size-4 text-[var(--color-danger)]" aria-hidden />
              ) : (
                <Wifi
                  className={
                    d.status === 'degraded'
                      ? 'size-4 text-[var(--color-warn)]'
                      : 'size-4 text-[var(--color-cta)]'
                  }
                  aria-hidden
                />
              )}
            </div>
          </motion.li>
        ))}
      </AnimatePresence>
    </ul>
  )
}
