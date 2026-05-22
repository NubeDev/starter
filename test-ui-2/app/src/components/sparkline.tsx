import { motion } from 'motion/react'

type Props = {
  data: number[]
  color?: string
  height?: number
  label?: string
}

export function Sparkline({ data, color = 'var(--color-cta)', height = 120, label }: Props) {
  const w = 600
  const h = height
  const pad = 8
  const min = Math.min(...data)
  const max = Math.max(...data)
  const range = Math.max(1, max - min)
  const step = (w - pad * 2) / Math.max(1, data.length - 1)

  const points = data.map((v, i) => {
    const x = pad + i * step
    const y = h - pad - ((v - min) / range) * (h - pad * 2)
    return [x, y] as const
  })

  const line = points.map(([x, y], i) => `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`).join(' ')
  const area = `${line} L${pad + (data.length - 1) * step},${h - pad} L${pad},${h - pad} Z`

  return (
    <div className="relative w-full">
      {label && (
        <div className="mb-2 flex items-center justify-between">
          <span className="font-mono text-xs uppercase tracking-wider text-[var(--color-muted)]">
            {label}
          </span>
          <span className="font-mono text-xs text-[var(--color-muted)]">
            min {min.toFixed(1)} · max {max.toFixed(1)}
          </span>
        </div>
      )}
      <svg viewBox={`0 0 ${w} ${h}`} className="h-auto w-full" role="img" aria-label={label ?? 'sparkline'}>
        <defs>
          <linearGradient id={`fade-${color}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={color} stopOpacity="0.35" />
            <stop offset="100%" stopColor={color} stopOpacity="0" />
          </linearGradient>
        </defs>
        {/* grid */}
        {[0.25, 0.5, 0.75].map((p) => (
          <line
            key={p}
            x1={pad}
            x2={w - pad}
            y1={pad + (h - pad * 2) * p}
            y2={pad + (h - pad * 2) * p}
            stroke="currentColor"
            className="text-[var(--color-border)]"
            strokeDasharray="2 4"
          />
        ))}
        <path d={area} fill={`url(#fade-${color})`} />
        <motion.path
          d={line}
          fill="none"
          stroke={color}
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 0.6, ease: 'easeOut' }}
          style={{ filter: `drop-shadow(0 0 6px ${color})` }}
        />
        {points.length > 0 && (
          <motion.circle
            cx={points[points.length - 1][0]}
            cy={points[points.length - 1][1]}
            r={4}
            fill={color}
            initial={{ scale: 0 }}
            animate={{ scale: [1, 1.4, 1] }}
            transition={{ duration: 1.4, repeat: Infinity }}
          />
        )}
      </svg>
    </div>
  )
}
