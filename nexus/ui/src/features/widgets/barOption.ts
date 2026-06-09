import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { chromeColor, seriesColor } from "@/features/widgets/palette";

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
  const { x, series, xKind } = widget.config.fields;
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
    legend: multi
      ? { top: 0, right: 0, type: "scroll", textStyle: { color: label }, itemWidth: 10, itemHeight: 10 }
      : undefined,
    xAxis: {
      type: "category",
      data: categories,
      axisLine: { lineStyle: { color: border } },
      axisLabel: timeFmt
        ? { color: label, formatter: (v: string | number) => timeFmt(v) }
        : { color: label },
    },
    yAxis: {
      type: "value",
      axisLabel: { color: label },
      splitLine: { lineStyle: { color: border, opacity: 0.4 } },
    },
    series: series.map((field, i) => {
      const color = seriesColor(field, i);
      return {
        type: "bar",
        name: field.label ?? field.value,
        itemStyle: { color, borderRadius: [3, 3, 0, 0] },
        data: data.points.map((p) => p[field.value] ?? null),
      };
    }),
  };
}
