import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { latestValue } from "@/features/widgets/scalar";
import { thresholdState } from "@/features/widgets/thresholdState";

// Maps the threshold verdict to an arc colour. Warn/crit pull from the
// theme's signal tokens so the gauge matches the rest of the UI; nominal
// uses the primary accent.
const STATE_COLOR = {
  ok: "hsl(var(--primary))",
  warn: "hsl(var(--chart-4))",
  crit: "hsl(var(--destructive))",
} as const;

// Builds the ECharts gauge option from a single-value panel. The arc
// colour reflects the value's threshold state (ascending or descending,
// via `thresholdState`). With no rows the gauge shows an empty dial
// rather than a fabricated reading (F0).
export function buildGaugeOption(
  widget: Widget,
  data: WidgetData,
): EChartsOption {
  const { min = 0, max = 100, decimals = 0, thresholds } = widget.config;
  const unit = widget.config.fields.series[0]?.unit ?? "";
  const value = latestValue(widget, data);
  const state =
    value == null ? "ok" : thresholdState(value, thresholds?.warn, thresholds?.crit);
  const color = STATE_COLOR[state];

  return {
    series: [
      {
        type: "gauge",
        min,
        max,
        startAngle: 215,
        endAngle: -35,
        progress: { show: true, width: 10, itemStyle: { color } },
        itemStyle: { color },
        axisLine: { lineStyle: { width: 10, color: [[1, "hsl(var(--muted))"]] } },
        axisTick: { show: false },
        splitLine: { show: false },
        axisLabel: { show: false },
        pointer: { show: value != null, itemStyle: { color } },
        anchor: { show: false },
        detail: {
          valueAnimation: true,
          formatter: (v: number) => `${v.toFixed(decimals)}${unit}`,
          color: "hsl(var(--foreground))",
          fontSize: 22,
          offsetCenter: [0, "40%"],
        },
        data: value == null ? [] : [{ value }],
      },
    ],
  };
}
