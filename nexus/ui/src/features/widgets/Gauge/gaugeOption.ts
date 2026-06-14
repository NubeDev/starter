import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { resolveField, resolveThresholdSteps } from "@/features/widgets/_shared/fieldConfig";
import { formatValue } from "@/features/widgets/_shared/formatValue";
import { chromeColor, stateColor } from "@/features/widgets/_shared/palette";
import { rampColor } from "@/features/widgets/_shared/rampColor";
import { latestValue } from "@/features/widgets/_shared/scalar";
import { thresholdState } from "@/features/widgets/_shared/thresholdState";

// Builds the ECharts gauge option from a single-value panel. The arc
// colour reflects the value's threshold state: a multi-step `fieldConfig`
// ramp when one is set, otherwise the legacy warn/crit `thresholdState`,
// resolved to a concrete colour ECharts can paint (`stateColor` reads the
// theme ramp). Unit/decimals/min/max come from the resolved field config
// so the Field-tab settings reach the dial. With no rows the gauge shows
// an empty dial rather than a fabricated reading (F0).
export function buildGaugeOption(
  widget: Widget,
  data: WidgetData,
): EChartsOption {
  const series = widget.config.fields.series[0];
  const field = series
    ? resolveField(series, widget.config)
    : {};
  const min = field.min ?? widget.config.min ?? 0;
  const max = field.max ?? widget.config.max ?? 100;
  const value = latestValue(widget, data);
  const steps = resolveThresholdSteps(widget.config);
  const ramped = value != null ? rampColor(value, steps) : undefined;
  const legacy = widget.config.thresholds;
  const state =
    value == null ? "ok" : thresholdState(value, legacy?.warn, legacy?.crit);
  const color = ramped ?? stateColor(state);

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
          // renders an empty arc (F0). Formatting (unit/decimals/mappings)
          // comes from the resolved field config.
          formatter: (v: number) =>
            Number.isFinite(v) ? formatValue(v, field).text : "",
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
