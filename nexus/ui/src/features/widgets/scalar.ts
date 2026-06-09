import type { Widget, WidgetData } from "@/data/types";

// Single-value panels (gauge, stat) read one number: the latest value of
// the first mapped series. Returns null when there are no rows so the
// widget can render an empty state instead of inventing a zero (F0).
export function latestValue(widget: Widget, data: WidgetData): number | null {
  const field = widget.config.fields.series[0];
  if (!field || data.points.length === 0) return null;
  const last = data.points[data.points.length - 1][field.value];
  return typeof last === "number" ? last : null;
}

// The value one step back, used to compute a stat's delta. Null when
// there is no prior point.
export function previousValue(widget: Widget, data: WidgetData): number | null {
  const field = widget.config.fields.series[0];
  if (!field || data.points.length < 2) return null;
  const prev = data.points[data.points.length - 2][field.value];
  return typeof prev === "number" ? prev : null;
}
