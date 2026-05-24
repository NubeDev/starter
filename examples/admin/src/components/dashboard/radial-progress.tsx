import { motion } from 'motion/react'
import { cn } from '@/lib/utils'

interface RadialProgressProps {
  value: number
  label: string
  subLabel?: string
  className?: string
}

export function RadialProgress({ value, label, subLabel, className }: RadialProgressProps) {
  const size = 180
  const stroke = 10
  const r = (size - stroke) / 2
  const c = 2 * Math.PI * r
  const offset = c - (value / 100) * c

  return (
    <div className={cn('glass relative overflow-hidden rounded-3xl p-[var(--pad-card)]', className)}>
      <div className="text-[11px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
        {label}
      </div>
      <div className="mt-4 flex items-center justify-center">
        <div className="relative" style={{ width: size, height: size }}>
          <svg width={size} height={size} className="-rotate-90">
            <circle
              cx={size / 2}
              cy={size / 2}
              r={r}
              fill="none"
              stroke="rgba(255,255,255,0.06)"
              strokeWidth={stroke}
            />
            <defs>
              <linearGradient id="radialGrad" x1="0" x2="1" y1="0" y2="1">
                <stop offset="0%" stopColor="#4ade80" />
                <stop offset="100%" stopColor="#67e8f9" />
              </linearGradient>
            </defs>
            <motion.circle
              cx={size / 2}
              cy={size / 2}
              r={r}
              fill="none"
              stroke="url(#radialGrad)"
              strokeWidth={stroke}
              strokeLinecap="round"
              strokeDasharray={c}
              initial={{ strokeDashoffset: c }}
              whileInView={{ strokeDashoffset: offset }}
              viewport={{ once: true }}
              transition={{ duration: 1.6, ease: [0.22, 1, 0.36, 1] }}
            />
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <div className="tabular text-4xl font-semibold tracking-tight text-[color:var(--color-text)]">
              {value}
              <span className="text-xl text-[color:var(--color-subtle)]">%</span>
            </div>
            {subLabel && (
              <div className="mt-1 text-[10px] uppercase tracking-[0.15em] text-[color:var(--color-subtle)]">
                {subLabel}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
