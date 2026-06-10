import { MetricCard } from "@nube/starter-ui-dashboard";

import type { Widget, WidgetData } from "@/data/types";
import { Empty } from "@/features/state/Empty";
import { computeStat } from "@/features/widgets/statDelta";
import { resolveField } from "@/features/widgets/fieldConfig";
import { formatValue } from "@/features/widgets/formatValue";

// Stat / KPI panel. Reuses the starter dashboard `MetricCard` tile (D1) — we
// map our `Widget`+`WidgetData` onto its props. Pure: `computeStat` derives the
// reading from typed props, and with no rows we render empty, never a
// fabricated zero (F0).
//
// The displayed text goes through `formatValue` (the same formatter tables use)
// so decimals, unit symbol, AND value mappings all apply to a stat — they used
// to be bypassed because the raw number was fed straight to MetricCard. The
// formatted string is passed as `display`; a value-mapping colour as `valueColor`.
export function Stat({ widget, data }: { widget: Widget; data: WidgetData }) {
  const stat = computeStat(widget, data);
  const series = widget.config.fields.series[0];
  const resolved = series ? resolveField(series, widget.config) : undefined;

  if (!stat) {
    // Honour the field's configured "No-value display"; fall back to "No data".
    return <Empty title={widget.title} description={resolved?.noValue ?? "No data"} />;
  }

  const formatted = formatValue(stat.value, resolved);
  return (
    <MetricCard
      label={widget.title}
      value={stat.value}
      display={formatted.text}
      valueColor={formatted.color ? `hsl(${formatted.color})` : undefined}
      delta={stat.deltaPct ?? undefined}
      spark={[...stat.spark]}
      accent="hsl(var(--primary))"
    />
  );
}
