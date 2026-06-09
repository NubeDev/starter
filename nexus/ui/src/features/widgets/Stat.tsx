import { MetricCard } from "@nube/starter-ui-dashboard";

import type { Widget, WidgetData } from "@/data/types";
import { Empty } from "@/features/state/Empty";
import { computeStat } from "@/features/widgets/statDelta";

// Stat / KPI panel. Reuses the starter dashboard `MetricCard` tile
// (D1) — we map our `Widget`+`WidgetData` onto its props rather than
// hand-rolling a number+sparkline. Pure: `computeStat` derives the
// reading from typed props, and with no rows we render empty, never a
// fabricated zero (F0).
export function Stat({ widget, data }: { widget: Widget; data: WidgetData }) {
  const stat = computeStat(widget, data);
  if (!stat) return <Empty title={widget.title} description="No data" />;

  return (
    <MetricCard
      label={widget.title}
      value={stat.value}
      suffix={stat.unit}
      delta={stat.deltaPct ?? undefined}
      spark={[...stat.spark]}
      accent="hsl(var(--primary))"
    />
  );
}
