import { motion, AnimatePresence } from 'motion/react'
import { useEffect, useState } from 'react'
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
  title: string
  meta: string
  time: string
  accent?: 'leaf' | 'aqua' | 'sun' | 'default'
}

const ITEMS: ActivityItem[] = [
  { icon: Leaf, title: 'Indoor air upgraded', meta: 'living room · AQI 8', time: 'just now', accent: 'leaf' },
  { icon: Droplet, title: 'Water filter swapped', meta: 'kitchen · 0.4µm cartridge', time: '2m', accent: 'aqua' },
  { icon: Sun, title: 'Solar peak detected', meta: '+ 4.2 kWh stored', time: '14m', accent: 'sun' },
  { icon: Sprout, title: 'New seedling sensed', meta: 'monstera · soil moist', time: '1h', accent: 'leaf' },
  { icon: Wind, title: 'CO₂ vented automatically', meta: 'bedroom · 740 → 480 ppm', time: '2h', accent: 'aqua' },
  { icon: Recycle, title: 'Greywater recycled', meta: '32 L diverted to garden', time: '3h', accent: 'leaf' },
]

export function ActivityFeed({ className }: { className?: string }) {
  // Slowly rotate the list to feel "live"
  const [start, setStart] = useState(0)
  useEffect(() => {
    const t = setInterval(() => setStart((s) => (s + 1) % ITEMS.length), 4500)
    return () => clearInterval(t)
  }, [])

  const visible = Array.from({ length: 5 }, (_, i) => ITEMS[(start + i) % ITEMS.length])

  return (
    <div className={cn('glass hairline relative overflow-hidden rounded-3xl p-[var(--pad-card)]', className)}>
      <div className="mb-6 flex items-center justify-between">
        <div className="text-[11px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
          Living signal
        </div>
        <span className="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.15em] text-[color:var(--color-leaf)]">
          <span className="relative flex h-1.5 w-1.5">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[color:var(--color-leaf)] opacity-75" />
            <span className="relative inline-flex h-1.5 w-1.5 rounded-full bg-[color:var(--color-leaf)]" />
          </span>
          streaming
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
                key={item.title + i}
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
                  <div className="truncate text-sm font-medium text-[color:var(--color-text)]">{item.title}</div>
                  <div className="truncate text-xs text-[color:var(--color-subtle)]">{item.meta}</div>
                </div>
                <div className="tabular shrink-0 text-[11px] text-[color:var(--color-subtle)]">
                  {i === 0 ? 'now' : item.time}
                </div>
              </motion.li>
            )
          })}
        </AnimatePresence>
      </ul>
    </div>
  )
}
