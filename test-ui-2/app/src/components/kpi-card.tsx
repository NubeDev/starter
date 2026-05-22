import { useEffect, useState } from 'react'
import { motion } from 'motion/react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

type Props = {
  title: string
  unit: string
  value: number
  delta?: number
  icon: React.ReactNode
  accent?: string
}

export function KpiCard({
  title,
  unit,
  value,
  delta = 0,
  icon,
  accent = 'var(--color-primary)',
}: Props) {
  const [display, setDisplay] = useState(value)
  useEffect(() => {
    const start = display
    const end = value
    const t0 = performance.now()
    const dur = 600
    let raf = 0
    const tick = (t: number) => {
      const p = Math.min(1, (t - t0) / dur)
      const eased = 1 - Math.pow(1 - p, 3)
      setDisplay(start + (end - start) * eased)
      if (p < 1) raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value])

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: [0.22, 1, 0.36, 1] }}
      whileHover={{ y: -2 }}
    >
      <Card hairline className="relative">
        {/* corner glow */}
        <div
          aria-hidden
          className="pointer-events-none absolute -top-16 -right-16 size-40 rounded-full opacity-30 blur-3xl"
          style={{ background: accent }}
        />
        <CardHeader>
          <CardTitle>{title}</CardTitle>
          <span
            className="flex size-7 items-center justify-center rounded-md"
            style={{ background: `${accent}22`, color: accent, boxShadow: `inset 0 0 0 1px ${accent}55` }}
          >
            {icon}
          </span>
        </CardHeader>
        <CardContent>
          <div className="flex items-baseline gap-1">
            <span className="text-3xl font-semibold tabular-nums tracking-tight">
              {display.toFixed(unit === '%' || unit === '°C' ? 1 : 0)}
            </span>
            <span className="text-sm text-[var(--color-muted)]">{unit}</span>
          </div>
          <div className="mt-1 font-mono text-[11px] text-[var(--color-muted)]">
            <span
              className={
                delta >= 0
                  ? 'text-zinc-200'
                  : 'text-[var(--color-danger)]'
              }
            >
              {delta >= 0 ? '▲' : '▼'} {Math.abs(delta).toFixed(1)}
              {unit === '%' ? 'pp' : unit}
            </span>{' '}
            vs 5m
          </div>

          {/* gradient sparkline-ish flourish at bottom */}
          <div
            aria-hidden
            className="mt-3 h-[3px] w-full rounded-full"
            style={{
              background: `linear-gradient(90deg, transparent, ${accent}, transparent)`,
              boxShadow: `0 0 12px ${accent}55`,
            }}
          />
        </CardContent>
      </Card>
    </motion.div>
  )
}
