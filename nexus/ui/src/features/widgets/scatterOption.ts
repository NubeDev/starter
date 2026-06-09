import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { chromeColor, seriesColor } from "@/features/widgets/palette";

// Builds the ECharts option for a scatter panel. Pure (F6). The `x`
// column is the horizontal value and each mapped series is plotted as
// [x, y] points against it. Unlike line/area the x-axis is a *value*
// axis, so non-numeric x rows are dropped rather than indexed — a scatter
// of categorical x makes no sense. With no usable rows it renders empty
// (F0). A time x-axis formats its labels through the injected formatter.
export function buildScatterOption(
  widget: Widget,
  data: WidgetData,
  opts: { formatX?: (value: string | number) => string } = {},
): EChartsOption {
  const { x, series, xKind } = widget.config.fields;
  const border = chromeColor("--border");
  const label = chromeColor("--muted-foreground");
  const multi = series.length > 1;
  const timeFmt = xKind === "time" && opts.formatX ? opts.formatX : undefined;

  return {
    grid: { left: 8, right: 14, top: multi ? 30 : 12, bottom: 6, containLabel: true },
    tooltip: { trigger: "item" },
    legend: multi
      ? { top: 0, right: 0, type: "scroll", textStyle: { color: label }, itemWidth: 10, itemHeight: 10 }
      : undefined,
    xAxis: {
      type: "value",
      axisLine: { lineStyle: { color: border } },
      splitLine: { lineStyle: { color: border, opacity: 0.4 } },
      axisLabel: timeFmt
        ? { color: label, formatter: (v: number) => timeFmt(v) }
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
        type: "scatter",
        name: field.label ?? field.value,
        symbolSize: 8,
        itemStyle: { color },
        // [x, y] pairs; rows whose x or y isn't numeric are skipped so the
        // value axes stay meaningful (no fabricated coordinates, F0).
        data: data.points
          .map((p) => {
            const xv = x ? p[x] : null;
            const yv = p[field.value];
            return typeof xv === "number" && typeof yv === "number"
              ? [xv, yv]
              : null;
          })
          .filter((pair): pair is [number, number] => pair !== null),
      };
    }),
  };
}
