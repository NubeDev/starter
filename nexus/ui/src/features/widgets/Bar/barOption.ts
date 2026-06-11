import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { legendFragment, yAxisFragment } from "@/features/widgets/_shared/cartesianChrome";
import { resolveField } from "@/features/widgets/_shared/fieldConfig";
import { chromeColor, seriesColor } from "@/features/widgets/_shared/palette";

// Builds the ECharts option for a bar panel from its typed field mapping
// and fetched rows. Pure: same inputs → same option, no fetching (F6).
// Shares the category-x / multi-series shape with the line builder; the
// difference is `type: "bar"` and no area fill, so it's a separate small
// builder rather than another flag on `buildLineOption`.
export function buildBarOption(
  widget: Widget,
  data: WidgetData,
  opts: {
    /** Region/preference-aware label formatter for a time x-axis,
     *  applied only when `fields.xKind === "time"`. Injected by the panel
     *  from `useDateTime()` so this builder stays pure (F6). */
    formatX?: (value: string | number) => string;
  } = {},
): EChartsOption {
  const { x, xKind } = widget.config.fields;
  // Resolve per-series overrides and drop hidden series (mirrors the line
  // builder so overrides behave consistently across chart families).
  const series = widget.config.fields.series
    .map((field, index) => ({ field, index, resolved: resolveField(field, widget.config) }))
    .filter((s) => !s.resolved.hidden);
  const categories = x
    ? data.points.map((p) => p[x] ?? "")
    : data.points.map((_, i) => i);

  const border = chromeColor("--border");
  const label = chromeColor("--muted-foreground");
  const multi = series.length > 1;
  const timeFmt = xKind === "time" && opts.formatX ? opts.formatX : undefined;

  return {
    grid: { left: 8, right: 14, top: multi ? 30 : 12, bottom: 6, containLabel: true },
    tooltip: { trigger: "axis", axisPointer: { type: "shadow" } },
    legend: legendFragment(widget.config.options, multi, label),
    xAxis: {
      type: "category",
      data: categories,
      axisLine: { lineStyle: { color: border } },
      axisLabel: timeFmt
        ? { color: label, formatter: (v: string | number) => timeFmt(v) }
        : { color: label },
    },
    yAxis: yAxisFragment(widget.config.options, border, label),
    series: series.map(({ field, index, resolved }) => {
      const color = seriesColor({ ...field, color: resolved.color ?? field.color }, index);
      return {
        type: "bar",
        name: resolved.displayName ?? field.label ?? field.value,
        itemStyle: { color, borderRadius: [3, 3, 0, 0] },
        data: data.points.map((p) => p[field.value] ?? null),
      };
    }),
  };
}
