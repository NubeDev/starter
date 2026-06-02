import { motion } from 'framer-motion'

// Sparkline trend — a smooth polyline over a deterministic demo series.
// `seed` keeps each tile's shape stable across renders (no Math.random).
export function LineWidget({
  title,
  unit,
  accent,
  seed = 0,
}: {
  title: string
  unit?: string | null
  accent: string
  seed?: number
}) {
  const points = series(seed)
  const max = Math.max(...points)
  const min = Math.min(...points)
  const span = max - min || 1
  const w = 100
  const h = 40
  const d = points
    .map((p, i) => {
      const x = (i / (points.length - 1)) * w
      const y = h - ((p - min) / span) * h
      return `${i === 0 ? 'M' : 'L'} ${x.toFixed(1)} ${y.toFixed(1)}`
    })
    .join(' ')
  const last = points[points.length - 1]
  return (
    <div className="flex h-full flex-col justify-between">
      <div className="flex items-baseline justify-between">
        <p className="label">{title}</p>
        <p className="text-lg font-bold text-ink">
          {last}
          {unit ? <span className="ml-0.5 text-xs text-ink-muted">{unit}</span> : null}
        </p>
      </div>
      <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" className="h-12 w-full">
        <motion.path
          d={d}
          fill="none"
          stroke={accent}
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 0.8, ease: 'easeOut' }}
        />
      </svg>
    </div>
  )
}

// Deterministic pseudo-series from a seed — stable demo shape, SSR-safe.
function series(seed: number): number[] {
  const out: number[] = []
  let v = 40 + (seed % 20)
  for (let i = 0; i < 16; i++) {
    v += Math.sin((i + seed) * 0.9) * 6 + Math.cos((i + seed) * 0.4) * 3
    out.push(Math.round(Math.max(5, Math.min(95, v))))
  }
  return out
}
