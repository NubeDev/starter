import * as React from 'react'
import { cn } from '@/lib/utils'

type Tone = 'ok' | 'warn' | 'danger' | 'info' | 'muted'

const TONE: Record<Tone, string> = {
  ok: 'bg-white/10 text-zinc-100 ring-1 ring-inset ring-white/20',
  warn: 'bg-[var(--color-warn)]/15 text-[var(--color-warn)] ring-1 ring-inset ring-[var(--color-warn)]/30',
  danger: 'bg-[var(--color-danger)]/15 text-[var(--color-danger)] ring-1 ring-inset ring-[var(--color-danger)]/30',
  info: 'bg-white/8 text-zinc-300 ring-1 ring-inset ring-white/15',
  muted: 'bg-[var(--color-surface-2)]/60 text-[var(--color-muted)] ring-1 ring-inset ring-[var(--color-border)]',
}

export function Badge({
  tone = 'muted',
  className,
  ...props
}: React.HTMLAttributes<HTMLSpanElement> & { tone?: Tone }) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[11px] font-mono uppercase tracking-wider',
        TONE[tone],
        className,
      )}
      {...props}
    />
  )
}

export function StatusDot({ tone = 'muted' }: { tone?: Tone }) {
  const color =
    tone === 'ok'
      ? '#e4e4e7' // zinc-200
      : tone === 'warn'
        ? 'var(--color-warn)'
        : tone === 'danger'
          ? 'var(--color-danger)'
          : tone === 'info'
            ? '#d4d4d8' // zinc-300
            : 'var(--color-muted)'
  return (
    <span
      aria-hidden
      className="inline-block size-2 rounded-full"
      style={{
        background: color,
        boxShadow: `0 0 10px ${color}`,
      }}
    />
  )
}
