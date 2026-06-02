import { motion } from 'framer-motion'

// Radial gauge — an SVG arc that fills to `value` over `[min,max]`.
// Demo value is passed in (no live ingest yet); deterministic, no Math.random.
export function GaugeWidget({
  title,
  unit,
  value = 0,
  min = 0,
  max = 100,
  accent,
}: {
  title: string
  unit?: string | null
  value?: number
  min?: number
  max?: number
  accent: string
}) {
  const pct = Math.max(0, Math.min(1, (value - min) / (max - min || 1)))
  const r = 38
  const circ = Math.PI * r // half circle
  return (
    <div className="flex flex-col items-center">
      <svg viewBox="0 0 100 60" className="w-full max-w-[160px]">
        <path
          d="M 8 56 A 42 42 0 0 1 92 56"
          fill="none"
          stroke="rgba(255,255,255,0.1)"
          strokeWidth="8"
          strokeLinecap="round"
        />
        <motion.path
          d="M 8 56 A 42 42 0 0 1 92 56"
          fill="none"
          stroke={accent}
          strokeWidth="8"
          strokeLinecap="round"
          strokeDasharray={circ}
          initial={{ strokeDashoffset: circ }}
          animate={{ strokeDashoffset: circ * (1 - pct) }}
          transition={{ type: 'spring', stiffness: 80, damping: 18 }}
        />
      </svg>
      <p className="-mt-2 text-2xl font-extrabold text-ink">
        {value}
        {unit ? <span className="ml-0.5 text-sm font-semibold text-ink-muted">{unit}</span> : null}
      </p>
      <p className="label mt-0.5">{title}</p>
    </div>
  )
}
