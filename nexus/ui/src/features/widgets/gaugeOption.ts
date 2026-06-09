import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { chromeColor, stateColor } from "@/features/widgets/palette";
import { latestValue } from "@/features/widgets/scalar";
import { thresholdState } from "@/features/widgets/thresholdState";

// Builds the ECharts gauge option from a single-value panel. The arc
// colour reflects the value's threshold state (ascending or descending,
// via `thresholdState`), resolved to a concrete colour ECharts can paint
// (`stateColor` reads the theme ramp). With no rows the gauge shows an
// empty dial rather than a fabricated reading (F0).
export function buildGaugeOption(
  widget: Widget,
  data: WidgetData,
): EChartsOption {
  const { min = 0, max = 100, decimals = 0, thresholds } = widget.config;
  const unit = widget.config.fields.series[0]?.unit ?? "";
  const value = latestValue(widget, data);
  const state =
    value == null ? "ok" : thresholdState(value, thresholds?.warn, thresholds?.crit);
  const color = stateColor(state);

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
        axisLine: { lineStyle: { width: 10, color: [[1, chromeColor("--muted")]] } },
        axisTick: { show: false },
        splitLine: { show: false },
        axisLabel: { show: false },
        pointer: { show: value != null, itemStyle: { color } },
        anchor: { show: false },
        detail: {
          valueAnimation: true,
          // No reading yet → blank detail rather than "NaN"; the dial still
          // renders an empty arc (F0).
          formatter: (v: number) =>
            Number.isFinite(v) ? `${v.toFixed(decimals)}${unit}` : "",
          color: chromeColor("--foreground"),
          fontSize: 22,
          offsetCenter: [0, "40%"],
        },
        // Always one data item so the series keeps a stable shape across
        // live updates. Going from a populated array to `[]` (or back)
        // makes ECharts' animation interpolate against an undefined element
        // and throw in `interpolate1DArray`; a constant-length array with a
        // NaN value avoids that while still reading as "no value".
        data: [{ value: value == null ? NaN : value }],
      },
    ],
  };
}
