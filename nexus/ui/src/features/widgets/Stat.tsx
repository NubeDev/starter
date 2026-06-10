import { MetricCard } from "@nube/starter-ui-dashboard";

import type { Widget, WidgetData } from "@/data/types";
import { Empty } from "@/features/state/Empty";
import { computeStat } from "@/features/widgets/statDelta";
import { unitSymbol } from "@/features/widgets/statSymbol";

// Stat / KPI panel. Reuses the starter dashboard `MetricCard` tile
// (D1) — we map our `Widget`+`WidgetData` onto its props rather than
// hand-rolling a number+sparkline. Pure: `computeStat` derives the
// reading from typed props, and with no rows we render empty, never a
// fabricated zero (F0). `MetricCard` animates a numeric value and renders
// the unit as a prefix/suffix, so the resolved unit's symbol is split out
// here rather than baked into the number.
export function Stat({ widget, data }: { widget: Widget; data: WidgetData }) {
  const stat = computeStat(widget, data);
  if (!stat) return <Empty title={widget.title} description="No data" />;

  const sym = unitSymbol(stat.unit);
  return (
    <MetricCard
      label={widget.title}
      value={stat.value}
      prefix={sym.prefix}
      suffix={sym.suffix}
      delta={stat.deltaPct ?? undefined}
      spark={[...stat.spark]}
      accent="hsl(var(--primary))"
    />
  );
}
