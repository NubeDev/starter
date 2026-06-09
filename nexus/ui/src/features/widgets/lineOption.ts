import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { seriesColor } from "@/features/widgets/palette";

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

  return {
    grid: { left: 8, right: 12, top: 12, bottom: 8, containLabel: true },
    tooltip: { trigger: "axis" },
    legend: series.length > 1 ? { bottom: 0, type: "scroll" } : undefined,
    xAxis: {
      type: "category",
      data: categories,
      boundaryGap: false,
      axisLine: { lineStyle: { color: "hsl(var(--border))" } },
    },
    yAxis: {
      type: "value",
      splitLine: { lineStyle: { color: "hsl(var(--border))", opacity: 0.4 } },
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
        areaStyle: opts.area ? { opacity: 0.18 } : undefined,
        data: data.points.map((p) => p[field.value] ?? null),
      };
    }),
  };
}
