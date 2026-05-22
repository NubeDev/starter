import { motion, useMotionValue, useTransform, animate } from 'motion/react'
import { useEffect } from 'react'

type Props = {
  value: number // 0..100
  label: string
  unit?: string
  size?: number
  /** Two-stop gradient for the arc. */
  from?: string
  to?: string
}

export function RadialGauge({
  value,
  label,
  unit = '%',
  size = 200,
  from = '#ffffff',
  to = 'var(--color-primary)',
}: Props) {
  const r = size / 2 - 14
  const c = 2 * Math.PI * r
  const v = Math.max(0, Math.min(100, value))

  const progress = useMotionValue(0)
  const dash = useTransform(progress, (p) => `${(c * p) / 100} ${c}`)
  const display = useTransform(progress, (p) => p.toFixed(0))

  useEffect(() => {
    const controls = animate(progress, v, {
      duration: 0.9,
      ease: [0.22, 1, 0.36, 1],
    })
    return () => controls.stop()
  }, [v, progress])

  const id = `g-${from.replace(/[^a-z]/gi, '')}-${to.replace(/[^a-z]/gi, '')}`

  return (
    <div className="flex flex-col items-center">
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="img" aria-label={label}>
        <defs>
          <linearGradient id={id} x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor={from} />
            <stop offset="100%" stopColor={to} />
          </linearGradient>
          <filter id={`${id}-glow`}>
            <feGaussianBlur stdDeviation="3" result="b" />
            <feMerge>
              <feMergeNode in="b" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {/* Track */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke="var(--color-border)"
          strokeWidth={10}
        />
        {/* Progress */}
        <motion.circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={`url(#${id})`}
          strokeWidth={10}
          strokeLinecap="round"
          strokeDasharray={dash}
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
          filter={`url(#${id}-glow)`}
        />

        {/* Tick marks */}
        {Array.from({ length: 60 }, (_, i) => {
          const ang = (i / 60) * 2 * Math.PI - Math.PI / 2
          const r1 = r + 6
          const r2 = r + (i % 5 === 0 ? 11 : 8)
          const x1 = size / 2 + Math.cos(ang) * r1
          const y1 = size / 2 + Math.sin(ang) * r1
          const x2 = size / 2 + Math.cos(ang) * r2
          const y2 = size / 2 + Math.sin(ang) * r2
          return (
            <line
              key={i}
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              stroke="var(--color-border-hi)"
              strokeWidth={i % 5 === 0 ? 1.2 : 0.6}
            />
          )
        })}
      </svg>

      <div className="mt-[-130px] flex flex-col items-center" aria-hidden>
        <motion.div className="font-mono text-4xl font-semibold tabular-nums tracking-tight">
          {display}
        </motion.div>
        <div className="font-mono text-xs text-[var(--color-muted)]">{unit}</div>
      </div>
      <div className="mt-2 font-mono text-[11px] uppercase tracking-wider text-[var(--color-muted)]">
        {label}
      </div>
    </div>
  )
}
