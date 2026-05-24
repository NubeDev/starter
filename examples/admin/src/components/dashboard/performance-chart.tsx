import { motion } from 'motion/react'
import { useIntl } from 'react-intl'
import { cn } from '@/lib/utils'

interface PerformanceChartProps {
  data: number[]
  labels: string[]
  className?: string
}

export function PerformanceChart({ data, labels, className }: PerformanceChartProps) {
  const intl = useIntl()
  const w = 720
  const h = 240
  const padX = 24
  const padY = 24
  const max = Math.max(...data) * 1.1
  const min = 0
  const range = max - min || 1

  const points = data.map((v, i) => {
    const x = padX + (i / (data.length - 1)) * (w - padX * 2)
    const y = h - padY - ((v - min) / range) * (h - padY * 2)
    return [x, y] as const
  })

  // Smooth path via cubic bezier
  const path = points.reduce((acc, [x, y], i) => {
    if (i === 0) return `M ${x} ${y}`
    const [px, py] = points[i - 1]
    const cx = (px + x) / 2
    return `${acc} C ${cx} ${py}, ${cx} ${y}, ${x} ${y}`
  }, '')

  const area = `${path} L ${points[points.length - 1][0]} ${h - padY} L ${points[0][0]} ${h - padY} Z`

  return (
    <div className={cn('glass relative overflow-hidden rounded-3xl p-[var(--pad-card)]', className)}>
      <div className="mb-4 flex items-start justify-between">
        <div>
          <div className="text-[11px] font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {intl.formatMessage({ id: 'chart.energyHarvested' })}
          </div>
          <div className="mt-1 flex items-baseline gap-2">
            <div className="tabular text-3xl font-semibold tracking-tight text-[color:var(--color-text)]">
              42.3<span className="text-base text-[color:var(--color-subtle)]">kWh</span>
            </div>
            <div className="text-sm text-[color:var(--color-leaf)]">↑ 12.4%</div>
          </div>
        </div>
        <div className="flex gap-1 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface-2)] p-1 text-[11px]">
          {['1D', '1W', '1M', '1Y'].map((p, i) => (
            <button
              key={p}
              className={cn(
                'cursor-pointer rounded-full px-3 py-1 transition-colors',
                i === 1
                  ? 'bg-[color:var(--color-text)] text-[color:var(--color-bg)]'
                  : 'text-[color:var(--color-subtle)] hover:text-[color:var(--color-text)]',
              )}
            >
              {p}
            </button>
          ))}
        </div>
      </div>

      <svg viewBox={`0 0 ${w} ${h}`} className="h-[240px] w-full overflow-visible">
        <defs>
          <linearGradient id="lineGrad" x1="0" x2="1" y1="0" y2="0">
            <stop offset="0%" stopColor="#67e8f9" />
            <stop offset="100%" stopColor="#4ade80" />
          </linearGradient>
          <linearGradient id="areaGrad" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stopColor="#4ade80" stopOpacity="0.25" />
            <stop offset="100%" stopColor="#4ade80" stopOpacity="0" />
          </linearGradient>
        </defs>

        {/* horizontal grid */}
        {[0.25, 0.5, 0.75].map((t) => (
          <line
            key={t}
            x1={padX}
            x2={w - padX}
            y1={padY + (h - padY * 2) * t}
            y2={padY + (h - padY * 2) * t}
            stroke="rgba(255,255,255,0.04)"
          />
        ))}

        <motion.path
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 1, delay: 0.4 }}
          d={area}
          fill="url(#areaGrad)"
        />
        <motion.path
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 1.6, ease: [0.22, 1, 0.36, 1] }}
          d={path}
          fill="none"
          stroke="url(#lineGrad)"
          strokeWidth={2}
          strokeLinecap="round"
        />

        {points.map(([x, y], i) => (
          <motion.circle
            key={i}
            initial={{ opacity: 0, scale: 0 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.3, delay: 0.8 + i * 0.04 }}
            cx={x}
            cy={y}
            r={3}
            fill="#06100c"
            stroke="#4ade80"
            strokeWidth={1.5}
          />
        ))}
      </svg>

      <div className="mt-2 flex justify-between px-6 text-[10px] uppercase tracking-[0.15em] text-[color:var(--color-subtle)]">
        {labels.map((l) => (
          <span key={l}>{l}</span>
        ))}
      </div>
    </div>
  )
}
