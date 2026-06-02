import { motion } from 'framer-motion'
import { BatteryFull, BatteryLow } from 'lucide-react'

// Battery level — horizontal fill bar + icon. Turns amber when low.
export function BatteryWidget({
  title,
  value = 0,
  accent,
}: {
  title: string
  value?: number
  accent: string
}) {
  const pct = Math.max(0, Math.min(100, value))
  const low = pct <= 20
  const color = low ? '#ffc24b' : accent
  const Icon = low ? BatteryLow : BatteryFull
  return (
    <div className="flex h-full flex-col justify-between">
      <div className="flex items-center justify-between">
        <p className="label">{title}</p>
        <Icon className="h-5 w-5" style={{ color }} />
      </div>
      <div>
        <p className="mb-1.5 text-2xl font-extrabold text-ink">{pct}%</p>
        <div className="h-2 w-full overflow-hidden rounded-full bg-white/10">
          <motion.div
            className="h-full rounded-full"
            style={{ backgroundColor: color }}
            initial={{ width: 0 }}
            animate={{ width: `${pct}%` }}
            transition={{ type: 'spring', stiffness: 90, damping: 18 }}
          />
        </div>
      </div>
    </div>
  )
}
