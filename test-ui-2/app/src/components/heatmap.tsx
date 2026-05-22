import { motion } from 'motion/react'

const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']

type Props = {
  /** matrix[day][hour] in 0..1 (uptime / utilisation / activity) */
  matrix: number[][]
}

function color(v: number): string {
  // Mono ramp: zinc-900 → zinc-50 (shadcn neutral)
  const stops = [
    [0.0, [24, 24, 27]],     // zinc-900
    [0.3, [63, 63, 70]],     // zinc-700
    [0.6, [113, 113, 122]],  // zinc-500
    [0.85, [212, 212, 216]], // zinc-300
    [1.0, [250, 250, 250]],  // zinc-50
  ] as const
  for (let i = 1; i < stops.length; i++) {
    const [t1, c1] = stops[i - 1]
    const [t2, c2] = stops[i]
    if (v <= t2) {
      const k = (v - t1) / (t2 - t1)
      const r = Math.round(c1[0] + (c2[0] - c1[0]) * k)
      const g = Math.round(c1[1] + (c2[1] - c1[1]) * k)
      const b = Math.round(c1[2] + (c2[2] - c1[2]) * k)
      return `rgb(${r} ${g} ${b})`
    }
  }
  return `rgb(${stops[stops.length - 1][1].join(' ')})`
}

export function Heatmap({ matrix }: Props) {
  return (
    <div className="w-full">
      <div className="mb-2 flex items-center justify-between font-mono text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
        <span>activity · 7d × 24h</span>
        <span className="flex items-center gap-1">
          low
          <span className="ml-1 inline-block h-2 w-24 rounded-full"
            style={{
              background:
                'linear-gradient(90deg, rgb(24 24 27), rgb(63 63 70), rgb(161 161 170), rgb(250 250 250))',
            }}
          />
          high
        </span>
      </div>
      <div className="grid grid-cols-[auto_1fr] gap-x-2">
        <div className="grid grid-rows-7 gap-1 pt-[2px]">
          {DAYS.map((d) => (
            <div
              key={d}
              className="font-mono text-[10px] uppercase leading-[14px] text-[var(--color-muted)]"
            >
              {d}
            </div>
          ))}
        </div>
        <div className="grid grid-rows-7 gap-1">
          {matrix.map((row, dy) => (
            <div key={dy} className="grid grid-cols-24" style={{ gridTemplateColumns: 'repeat(24, minmax(0,1fr))' }}>
              {row.map((v, hx) => (
                <motion.div
                  key={hx}
                  className="aspect-square rounded-[2px]"
                  initial={{ opacity: 0, scale: 0.6 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ delay: (dy * 24 + hx) * 0.002, duration: 0.25 }}
                  style={{ background: color(v) }}
                  title={`${DAYS[dy]} ${hx}:00 · ${(v * 100).toFixed(0)}%`}
                />
              ))}
            </div>
          ))}
        </div>
      </div>
      <div className="mt-2 grid grid-cols-[auto_1fr] gap-x-2">
        <div className="w-7" />
        <div className="grid font-mono text-[9px] text-[var(--color-muted)]" style={{ gridTemplateColumns: 'repeat(24, minmax(0,1fr))' }}>
          {Array.from({ length: 24 }, (_, i) => (
            <span key={i} className="text-center">
              {i % 3 === 0 ? i : ''}
            </span>
          ))}
        </div>
      </div>
    </div>
  )
}
