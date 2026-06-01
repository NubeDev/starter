import { Hash } from 'lucide-react'

// Monotonic counter (pulses, kWh ticks, etc.) — monospace-ish numeric readout.
export function CounterWidget({
  title,
  unit,
  value = 0,
  accent,
}: {
  title: string
  unit?: string | null
  value?: number
  accent: string
}) {
  return (
    <div className="flex h-full flex-col justify-between">
      <div className="flex items-center justify-between">
        <p className="label">{title}</p>
        <Hash className="h-5 w-5" style={{ color: accent }} />
      </div>
      <p className="font-mono text-3xl font-bold tabular-nums text-ink">
        {value.toLocaleString()}
        {unit ? <span className="ml-1 text-sm font-semibold text-ink-muted">{unit}</span> : null}
      </p>
    </div>
  )
}
