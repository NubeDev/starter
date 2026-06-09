import type { Trend, Widget, WidgetData } from "@/data/types";
import { latestValue, previousValue } from "@/features/widgets/scalar";

export interface StatReading {
  value: number;
  /** Percent change vs the prior point; null when there's only one. */
  deltaPct: number | null;
  trend: Trend;
  unit?: string;
  decimals: number;
  /** The series values, for a sparkline. */
  spark: ReadonlyArray<number>;
}

// Computes a stat/KPI reading: the latest value, its percent delta
// against the previous point, and the spark series. Returns null when
// there is no data so the widget renders an empty state (F0).
export function computeStat(
  widget: Widget,
  data: WidgetData,
): StatReading | null {
  const value = latestValue(widget, data);
  if (value == null) return null;
  const prev = previousValue(widget, data);
  const field = widget.config.fields.series[0];

  const deltaPct =
    prev == null || prev === 0 ? null : ((value - prev) / Math.abs(prev)) * 100;
  const trend: Trend =
    deltaPct == null || deltaPct === 0 ? "flat" : deltaPct > 0 ? "up" : "down";

  return {
    value,
    deltaPct,
    trend,
    unit: field?.unit,
    decimals: widget.config.decimals ?? 0,
    spark: data.points
      .map((p) => p[field.value])
      .filter((n): n is number => typeof n === "number"),
  };
}
