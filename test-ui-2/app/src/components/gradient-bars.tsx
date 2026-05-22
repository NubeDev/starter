import { motion } from 'motion/react'

type Series = {
  name: string
  color: string
  data: number[]
}

type Props = {
  labels: string[]
  series: Series[]
  height?: number
}

export function GradientBars({ labels, series, height = 240 }: Props) {
  const w = 720
  const h = height
  const padL = 36
  const padR = 8
  const padT = 12
  const padB = 28
  const innerW = w - padL - padR
  const innerH = h - padT - padB

  const groups = labels.length
  const seriesCount = series.length
  const groupGap = 14
  const barGap = 3
  const groupW = (innerW - groupGap * (groups - 1)) / groups
  const barW = (groupW - barGap * (seriesCount - 1)) / seriesCount

  const max = Math.max(1, ...series.flatMap((s) => s.data))
  const ticks = 4

  return (
    <div className="relative w-full">
      <div className="mb-3 flex flex-wrap items-center gap-3">
        {series.map((s) => (
          <span
            key={s.name}
            className="inline-flex items-center gap-2 font-mono text-[11px] uppercase tracking-wider text-[var(--color-muted)]"
          >
            <span
              aria-hidden
              className="inline-block size-2 rounded-full"
              style={{ background: s.color, boxShadow: `0 0 8px ${s.color}` }}
            />
            {s.name}
          </span>
        ))}
      </div>

      <svg
        viewBox={`0 0 ${w} ${h}`}
        className="h-auto w-full"
        role="img"
        aria-label="Grouped bar chart"
      >
        <defs>
          {series.map((s, i) => (
            <linearGradient key={i} id={`bar-${i}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={s.color} stopOpacity="1" />
              <stop offset="100%" stopColor={s.color} stopOpacity="0.25" />
            </linearGradient>
          ))}
        </defs>

        {/* Y grid + labels */}
        {Array.from({ length: ticks + 1 }, (_, i) => {
          const y = padT + (innerH / ticks) * i
          const value = Math.round((max * (ticks - i)) / ticks)
          return (
            <g key={i}>
              <line
                x1={padL}
                x2={w - padR}
                y1={y}
                y2={y}
                stroke="currentColor"
                className="text-[var(--color-border)]"
                strokeDasharray={i === ticks ? '0' : '2 4'}
              />
              <text
                x={padL - 6}
                y={y + 3}
                textAnchor="end"
                className="fill-[var(--color-muted)] font-mono"
                style={{ fontSize: 9 }}
              >
                {value}
              </text>
            </g>
          )
        })}

        {/* Bars */}
        {labels.map((label, gi) => {
          const gx = padL + gi * (groupW + groupGap)
          return (
            <g key={label}>
              {series.map((s, si) => {
                const v = s.data[gi] ?? 0
                const bh = (v / max) * innerH
                const bx = gx + si * (barW + barGap)
                const by = padT + (innerH - bh)
                return (
                  <motion.rect
                    key={si}
                    x={bx}
                    width={barW}
                    rx={3}
                    fill={`url(#bar-${si})`}
                    initial={{ y: padT + innerH, height: 0 }}
                    animate={{ y: by, height: bh }}
                    transition={{
                      duration: 0.6,
                      delay: gi * 0.04 + si * 0.05,
                      ease: [0.22, 1, 0.36, 1],
                    }}
                    style={{ filter: `drop-shadow(0 0 6px ${s.color}55)` }}
                  />
                )
              })}
              <text
                x={gx + groupW / 2}
                y={h - 8}
                textAnchor="middle"
                className="fill-[var(--color-muted)] font-mono"
                style={{ fontSize: 9 }}
              >
                {label}
              </text>
            </g>
          )
        })}
      </svg>
    </div>
  )
}
