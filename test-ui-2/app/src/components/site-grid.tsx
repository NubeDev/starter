import { motion } from 'motion/react'

type Site = {
  name: string
  code: string
  online: number
  total: number
  status: 'ok' | 'warn' | 'danger'
}

const SITES: Site[] = [
  { name: 'Plant A', code: 'PLT-A', online: 42, total: 48, status: 'ok' },
  { name: 'Plant B', code: 'PLT-B', online: 18, total: 24, status: 'warn' },
  { name: 'Yard East', code: 'YRD-E', online: 6, total: 12, status: 'danger' },
  { name: 'DC Edge', code: 'DC-01', online: 16, total: 16, status: 'ok' },
  { name: 'Coldroom', code: 'CLD-1', online: 7, total: 8, status: 'warn' },
  { name: 'Roof Mesh', code: 'RF-12', online: 11, total: 12, status: 'ok' },
]

const TONE = {
  ok: { ring: 'var(--color-ok)', glow: 'rgba(16,185,129,0.30)' },
  warn: { ring: 'var(--color-warn)', glow: 'rgba(245,158,11,0.30)' },
  danger: { ring: 'var(--color-danger)', glow: 'rgba(239,68,68,0.30)' },
}

export function SiteGrid() {
  return (
    <ul className="grid grid-cols-2 gap-2 sm:grid-cols-3">
      {SITES.map((s, i) => {
        const pct = (s.online / s.total) * 100
        const t = TONE[s.status]
        return (
          <motion.li
            key={s.code}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: i * 0.04 }}
            className="relative overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-surface-2)]/40 p-3"
          >
            <div
              aria-hidden
              className="absolute inset-x-0 top-0 h-px"
              style={{ background: `linear-gradient(90deg, transparent, ${t.ring}, transparent)` }}
            />
            <div className="flex items-start justify-between">
              <div>
                <div className="text-sm font-medium">{s.name}</div>
                <div className="font-mono text-[10px] uppercase tracking-wider text-[var(--color-muted)]">
                  {s.code}
                </div>
              </div>
              <span
                aria-hidden
                className="size-2 rounded-full"
                style={{ background: t.ring, boxShadow: `0 0 10px ${t.glow}` }}
              />
            </div>
            <div className="mt-3 flex items-baseline gap-1">
              <span className="font-mono text-xl font-semibold tabular-nums">{s.online}</span>
              <span className="font-mono text-xs text-[var(--color-muted)]">/ {s.total}</span>
            </div>
            <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-[var(--color-border)]/60">
              <motion.div
                initial={{ width: 0 }}
                animate={{ width: `${pct}%` }}
                transition={{ duration: 0.6, delay: 0.1 + i * 0.04, ease: [0.22, 1, 0.36, 1] }}
                className="h-full rounded-full"
                style={{
                  background: `linear-gradient(90deg, ${t.ring}, rgba(255,255,255,0.15))`,
                  boxShadow: `0 0 8px ${t.glow}`,
                }}
              />
            </div>
          </motion.li>
        )
      })}
    </ul>
  )
}
