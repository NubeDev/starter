import { motion, AnimatePresence } from 'motion/react'
import { useEffect, useState } from 'react'
import { useIntl } from 'react-intl'
import {
  Leaf,
  Droplet,
  Wind,
  Sun,
  Recycle,
  Sprout,
  type LucideIcon,
} from 'lucide-react'
import { cn } from '@/lib/utils'

interface ActivityItem {
  icon: LucideIcon
  titleKey: string
  metaKey: string
  time: string
  accent?: 'leaf' | 'aqua' | 'sun' | 'default'
}

const ITEMS: ActivityItem[] = [
  { icon: Leaf,    titleKey: 'activity.item.airUpgraded.title', metaKey: 'activity.item.airUpgraded.meta', time: '0m',  accent: 'leaf' },
  { icon: Droplet, titleKey: 'activity.item.waterFilter.title', metaKey: 'activity.item.waterFilter.meta', time: '2m',  accent: 'aqua' },
  { icon: Sun,     titleKey: 'activity.item.solarPeak.title',   metaKey: 'activity.item.solarPeak.meta',   time: '14m', accent: 'sun' },
  { icon: Sprout,  titleKey: 'activity.item.seedling.title',    metaKey: 'activity.item.seedling.meta',    time: '1h',  accent: 'leaf' },
  { icon: Wind,    titleKey: 'activity.item.co2Vented.title',   metaKey: 'activity.item.co2Vented.meta',   time: '2h',  accent: 'aqua' },
  { icon: Recycle, titleKey: 'activity.item.greywater.title',   metaKey: 'activity.item.greywater.meta',   time: '3h',  accent: 'leaf' },
]

export function ActivityFeed({ className }: { className?: string }) {
  // Slowly rotate the list to feel "live"
  const [start, setStart] = useState(0)
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  useEffect(() => {
    const t = setInterval(() => setStart((s) => (s + 1) % ITEMS.length), 4500)
    return () => clearInterval(t)
  }, [])

  const visible = Array.from({ length: 5 }, (_, i) => ITEMS[(start + i) % ITEMS.length])

  return (
    <div className={cn('glass hairline relative overflow-hidden rounded-3xl p-[var(--pad-card)]', className)}>
      <div className="mb-6 flex items-center justify-between">
        <div className="text-[11px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
          {tr('activity.title')}
        </div>
        <span className="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.15em] text-[color:var(--color-leaf)]">
          <span className="relative flex h-1.5 w-1.5">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[color:var(--color-leaf)] opacity-75" />
            <span className="relative inline-flex h-1.5 w-1.5 rounded-full bg-[color:var(--color-leaf)]" />
          </span>
          {tr('activity.streaming')}
        </span>
      </div>

      <ul className="relative space-y-1">
        <AnimatePresence initial={false} mode="popLayout">
          {visible.map((item, i) => {
            const Icon = item.icon
            const accent =
              item.accent === 'leaf'
                ? 'bg-[color:var(--color-leaf)]/10 text-[color:var(--color-leaf)] ring-[color:var(--color-leaf)]/25'
                : item.accent === 'aqua'
                ? 'bg-[color:var(--color-aqua)]/10 text-[color:var(--color-aqua)] ring-[color:var(--color-aqua)]/25'
                : item.accent === 'sun'
                ? 'bg-[color:var(--color-sun)]/10 text-[color:var(--color-sun)] ring-[color:var(--color-sun)]/25'
                : 'bg-[color:var(--color-surface-2)] text-[color:var(--color-text)] ring-[color:var(--color-border)]'
            return (
              <motion.li
                key={item.titleKey + i}
                layout
                initial={{ opacity: 0, y: -12 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, x: 30 }}
                transition={{ duration: 0.55, ease: [0.22, 1, 0.36, 1] }}
                className="group flex cursor-default items-center gap-4 rounded-2xl px-3 py-3 transition-colors hover:bg-[color:var(--color-surface-2)]/40"
              >
                <div className={cn('flex h-9 w-9 items-center justify-center rounded-xl ring-1', accent)}>
                  <Icon className="h-4 w-4" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium text-[color:var(--color-text)]">{tr(item.titleKey)}</div>
                  <div className="truncate text-xs text-[color:var(--color-subtle)]">{tr(item.metaKey)}</div>
                </div>
                <div className="tabular shrink-0 text-[11px] text-[color:var(--color-subtle)]">
                  {i === 0 ? tr('activity.now') : item.time}
                </div>
              </motion.li>
            )
          })}
        </AnimatePresence>
      </ul>
    </div>
  )
}
