import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { chromeColor } from "@/features/widgets/_shared/palette";

// Builds the ECharts option for a heatmap panel. Pure (F6). A heatmap is
// a 2D grid: the `x` column is the x-axis category, the *first* series
// column is the y-axis category, and the *second* series column is the
// cell value (intensity). This mirrors the field-mapping vocabulary — no
// new config shape — at the cost of needing two series mapped; with
// fewer than two it renders an empty grid rather than guessing (F0).
//
// The colour ramp is read off the theme's `--chart-1` accent so the
// heat scale tracks the brand; the panel re-renders on dark/light switch
// to rebuild it (see the line/area panels for the rationale).
export function buildHeatmapOption(
  widget: Widget,
  data: WidgetData,
): EChartsOption {
  const { x, series } = widget.config.fields;
  const yField = series[0];
  const valField = series[1];
  const label = chromeColor("--muted-foreground");
  const border = chromeColor("--border");

  // Categorical axes are the distinct values in declaration order.
  const xCats = x ? distinct(data.points.map((p) => str(p[x]))) : [];
  const yCats = yField ? distinct(data.points.map((p) => str(p[yField.value]))) : [];
  const xIndex = indexOf(xCats);
  const yIndex = indexOf(yCats);

  const cells =
    x && yField && valField
      ? data.points
          .map((p) => {
            const xi = xIndex.get(str(p[x]));
            const yi = yIndex.get(str(p[yField.value]));
            const v = p[valField.value];
            return xi != null && yi != null && typeof v === "number"
              ? [xi, yi, v]
              : null;
          })
          .filter((c): c is [number, number, number] => c !== null)
      : [];

  const values = cells.map((c) => c[2]);
  const min = values.length ? Math.min(...values) : 0;
  const max = values.length ? Math.max(...values) : 1;

  return {
    grid: { left: 8, right: 14, top: 12, bottom: 40, containLabel: true },
    tooltip: { position: "top" },
    xAxis: {
      type: "category",
      data: xCats,
      axisLine: { lineStyle: { color: border } },
      axisLabel: { color: label },
      splitArea: { show: true },
    },
    yAxis: {
      type: "category",
      data: yCats,
      axisLine: { lineStyle: { color: border } },
      axisLabel: { color: label },
      splitArea: { show: true },
    },
    visualMap: {
      min,
      max: max === min ? min + 1 : max,
      calculable: true,
      orient: "horizontal",
      left: "center",
      bottom: 0,
      textStyle: { color: label },
      // A single-hue ramp from the muted track to the brand accent.
      inRange: { color: [chromeColor("--muted"), chromeColor("--chart-1")] },
    },
    series: [
      {
        type: "heatmap",
        data: cells,
        label: { show: false },
        emphasis: { itemStyle: { shadowBlur: 8, shadowColor: "rgba(0,0,0,0.4)" } },
      },
    ],
  };
}

function str(v: unknown): string {
  return v == null ? "" : String(v);
}

function distinct(values: string[]): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const v of values) {
    if (!seen.has(v)) {
      seen.add(v);
      out.push(v);
    }
  }
  return out;
}

function indexOf(cats: string[]): Map<string, number> {
  return new Map(cats.map((c, i) => [c, i]));
}
