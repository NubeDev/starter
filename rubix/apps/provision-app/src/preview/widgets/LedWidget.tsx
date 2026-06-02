// Status LED — a glowing dot for a boolean/discrete point (on/off, ok/fault).
export function LedWidget({
  title,
  on = false,
  accent,
}: {
  title: string
  on?: boolean
  accent: string
}) {
  const color = on ? accent : '#7c8a8a'
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3">
      <span
        className="h-7 w-7 rounded-full"
        style={{
          backgroundColor: color,
          boxShadow: on ? `0 0 18px 2px ${accent}` : 'none',
        }}
      />
      <p className="label">{title}</p>
      <p className="text-sm font-semibold text-ink-variant">{on ? 'On' : 'Off'}</p>
    </div>
  )
}
