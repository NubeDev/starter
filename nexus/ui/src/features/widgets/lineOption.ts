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
  opts: {
    area: boolean;
    /** When the x column is a time axis (`fields.xKind === "time"`),
     *  this renders each raw x value as a region/preference-aware date.
     *  Injected by the panel component from `useDateTime()` so this
     *  builder stays pure and off the React tree (F6). Omitted → raw
     *  category labels. */
    formatX?: (value: string | number) => string;
  },
): EChartsOption {
  const { x, series, xKind } = widget.config.fields;
  const categories = x ? data.points.map((p) => p[x] ?? "") : data.points.map((_, i) => i);

  const border = chromeColor("--border");
  const label = chromeColor("--muted-foreground");
  const multi = series.length > 1;

  // Only format when the column is declared a time axis *and* a
  // formatter was supplied; otherwise labels stay raw (existing
  // string-label charts are untouched).
  const timeFmt =
    xKind === "time" && opts.formatX ? opts.formatX : undefined;

  return {
    // Top padding leaves room for the legend so it never sits over the
    // plot; the x-axis labels get bottom room of their own.
    grid: { left: 8, right: 14, top: multi ? 30 : 12, bottom: 6, containLabel: true },
    tooltip: timeFmt
      ? {
          trigger: "axis",
          // ECharts passes the category (raw x value) as `axisValueLabel`
          // on the axis-trigger params; format the tooltip header to
          // match the axis. The shipped element type omits the axis
          // fields, so narrow locally to what we read.
          formatter: (params) => {
            type AxisItem = {
              axisValueLabel?: string;
              axisValue?: string | number;
              marker?: string;
              seriesName?: string;
              value?: unknown;
            };
            const list = (Array.isArray(params) ? params : [params]) as AxisItem[];
            const head = list[0]?.axisValueLabel ?? list[0]?.axisValue ?? "";
            const rows = list
              .map((p) => `${p.marker ?? ""}${p.seriesName}: ${p.value ?? "—"}`)
              .join("<br/>");
            return `${timeFmt(head)}<br/>${rows}`;
          },
        }
      : { trigger: "axis" },
    legend: multi
      ? { top: 0, right: 0, type: "scroll", textStyle: { color: label }, itemWidth: 10, itemHeight: 10 }
      : undefined,
    xAxis: {
      type: "category",
      data: categories,
      boundaryGap: false,
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
