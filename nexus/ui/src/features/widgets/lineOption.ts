import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { chromeColor, seriesColor, withAlpha } from "@/features/widgets/palette";

// Builds the ECharts option for a line or area panel from its typed
// field mapping and fetched rows. Pure: same inputs → same option, no
// fetching, no side effects (F6). The area flag is the only difference
// between the two panel types, so they share this builder.
export function buildLineOption(
  widget: Widget,
  data: WidgetData,
  opts: { area: boolean },
): EChartsOption {
  const { x, series } = widget.config.fields;
  const categories = x ? data.points.map((p) => p[x] ?? "") : data.points.map((_, i) => i);

  const border = chromeColor("--border");
  const label = chromeColor("--muted-foreground");
  const multi = series.length > 1;

  return {
    // Top padding leaves room for the legend so it never sits over the
    // plot; the x-axis labels get bottom room of their own.
    grid: { left: 8, right: 14, top: multi ? 30 : 12, bottom: 6, containLabel: true },
    tooltip: { trigger: "axis" },
    legend: multi
      ? { top: 0, right: 0, type: "scroll", textStyle: { color: label }, itemWidth: 10, itemHeight: 10 }
      : undefined,
    xAxis: {
      type: "category",
      data: categories,
      boundaryGap: false,
      axisLine: { lineStyle: { color: border } },
      axisLabel: { color: label },
    },
    yAxis: {
      type: "value",
      axisLabel: { color: label },
      splitLine: { lineStyle: { color: border, opacity: 0.4 } },
    },
    series: series.map((field, i) => {
      const color = seriesColor(field, i);
      return {
        type: "line",
        name: field.label ?? field.value,
        showSymbol: false,
        smooth: true,
        lineStyle: { color, width: 2 },
        itemStyle: { color },
        // A vertical fade from the line colour to transparent reads as
        // depth without muddying the plot — the mock's signature fill.
        areaStyle: opts.area
          ? {
              opacity: 0.9,
              color: {
                type: "linear",
                x: 0,
                y: 0,
                x2: 0,
                y2: 1,
                colorStops: [
                  { offset: 0, color: withAlpha(color, 0.35) },
                  { offset: 1, color: withAlpha(color, 0) },
                ],
              },
            }
          : undefined,
        data: data.points.map((p) => p[field.value] ?? null),
      };
    }),
  };
}
