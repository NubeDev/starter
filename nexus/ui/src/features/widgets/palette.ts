import type { SeriesField } from "@/data/types";

// The default series palette, in CSS-var form so a series with no
// explicit colour inherits the theme's chart ramp (set in index.css)
// rather than a hardcoded hex (F6: panels are theme-described). ECharts
// resolves `var(--chart-N)` against the chart container at paint time.
const CHART_VARS = [
  "var(--chart-1)",
  "var(--chart-2)",
  "var(--chart-3)",
  "var(--chart-4)",
  "var(--chart-5)",
] as const;

/** Resolve a series' colour: an explicit hsl config wins; otherwise the
 *  palette slot for its index, wrapping past five series. */
export function seriesColor(field: SeriesField, index: number): string {
  if (field.color) return `hsl(${field.color})`;
  return CHART_VARS[index % CHART_VARS.length];
}
