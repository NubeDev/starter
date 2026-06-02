// Big-number stat tile with a corner glow puddle — the GlanceTile pattern
// repurposed for a device reading.
export function StatWidget({
  title,
  unit,
  value = 0,
  accent,
}: {
  title: string
  unit?: string | null
  value?: number | string
  accent: string
}) {
  return (
    <div className="relative h-full overflow-hidden">
      <div
        className="absolute -bottom-8 -right-8 h-24 w-24 rounded-full blur-2xl"
        style={{ backgroundColor: accent, opacity: 0.25 }}
      />
      <div className="relative flex h-full flex-col justify-between">
        <p className="label">{title}</p>
        <p className="text-3xl font-extrabold text-ink">
          {value}
          {unit ? <span className="ml-1 text-base font-semibold text-ink-muted">{unit}</span> : null}
        </p>
      </div>
    </div>
  )
}
