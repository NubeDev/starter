import { Activity, BellRing } from 'lucide-react'
import { Toggle } from '../components/FormKit'
import { useLook } from '../theme/useLook'

// Trending / Alarming switches, pre-filled from template defaults upstream.
export function TogglesStep({
  trend,
  alarm,
  onTrend,
  onAlarm,
}: {
  trend: boolean
  alarm: boolean
  onTrend: (v: boolean) => void
  onAlarm: (v: boolean) => void
}) {
  const look = useLook()
  return (
    <div className="flex flex-col gap-3">
      <Row
        icon={Activity}
        title="Trending"
        blurb="Record point history for charts"
        on={trend}
        onToggle={() => onTrend(!trend)}
        accent={look.accent}
      />
      <Row
        icon={BellRing}
        title="Alarming"
        blurb="Arm template alarm thresholds"
        on={alarm}
        onToggle={() => onAlarm(!alarm)}
        accent={look.accent}
      />
    </div>
  )
}

function Row({
  icon: Icon,
  title,
  blurb,
  on,
  onToggle,
  accent,
}: {
  icon: typeof Activity
  title: string
  blurb: string
  on: boolean
  onToggle: () => void
  accent: string
}) {
  return (
    <div className="glass flex items-center gap-3 rounded-2xl p-4">
      <div className="grid h-10 w-10 place-items-center rounded-xl" style={{ backgroundColor: `${accent}22` }}>
        <Icon className="h-5 w-5" style={{ color: accent }} />
      </div>
      <div className="min-w-0 flex-1">
        <p className="font-semibold text-ink">{title}</p>
        <p className="text-xs text-ink-muted">{blurb}</p>
      </div>
      <Toggle on={on} onToggle={onToggle} accent={accent} label={title} />
    </div>
  )
}
