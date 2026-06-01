import type { WidgetKind } from '../../api/bc-types'
import { GlassCard } from '../../components/ui'
import { GaugeWidget } from './GaugeWidget'
import { StatWidget } from './StatWidget'
import { BatteryWidget } from './BatteryWidget'
import { CounterWidget } from './CounterWidget'
import { LedWidget } from './LedWidget'
import { ToggleWidget } from './ToggleWidget'
import { LineWidget } from './LineWidget'

// The renderer switchboard: maps a widget enum → its component, inside a glass
// tile. `bc_widgets` rows say "render `gauge` for point X"; this mounts it.
// Demo values are deterministic from `seed` (no live ingest yet).
export function WidgetTile({
  widget,
  title,
  unit,
  accent,
  seed = 0,
}: {
  widget: WidgetKind | string
  title: string
  unit?: string | null
  accent: string
  seed?: number
}) {
  // Stable per-tile demo reading derived from the seed.
  const demo = 20 + ((seed * 37) % 70)
  const span = widget === 'line' || widget === 'gauge' ? 'col-span-2' : ''

  return (
    <GlassCard className={`min-h-32 p-4 ${span}`}>
      {render(widget, { title, unit, accent, seed, demo })}
    </GlassCard>
  )
}

function render(
  widget: WidgetKind | string,
  p: { title: string; unit?: string | null; accent: string; seed: number; demo: number },
) {
  switch (widget) {
    case 'gauge':
      return <GaugeWidget title={p.title} unit={p.unit} value={p.demo} accent={p.accent} />
    case 'battery':
      return <BatteryWidget title={p.title} value={p.demo} accent={p.accent} />
    case 'counter':
      return <CounterWidget title={p.title} unit={p.unit} value={p.demo * 128} accent={p.accent} />
    case 'led':
      return <LedWidget title={p.title} on={p.demo > 45} accent={p.accent} />
    case 'toggle':
      return <ToggleWidget title={p.title} on={p.demo > 45} accent={p.accent} />
    case 'line':
      return <LineWidget title={p.title} unit={p.unit} accent={p.accent} seed={p.seed} />
    case 'stat':
    default:
      return <StatWidget title={p.title} unit={p.unit} value={p.demo} accent={p.accent} />
  }
}
