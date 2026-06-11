import type { EChartsOption } from "echarts";

import type { QueryResponse } from "@/api/types";
import { chromeColor, seriesColor } from "@/features/widgets/_shared/palette";

// Build an ECharts line option straight from a QueryResponse so the Workbench
// can chart a transform's output without the dashboard Widget model. The x axis
// is the first timestamp column if the result has one, else the row index; the
// series are every numeric column (`int` / `float`). Pure: same response →
// same option (F6). Reuses the theme-resolved palette helpers so colours track
// the brand, exactly like the dashboard charts.
const NUMERIC = new Set(["int", "float"]);

export function buildResultChartOption(result: QueryResponse): EChartsOption {
  const rows = result.rows as Record<string, unknown>[];
  const timeCol = result.columns.find((c) => c.type === "timestamp");
  const numericCols = result.columns.filter((c) => NUMERIC.has(c.type));

  const categories = timeCol
    ? rows.map((r) => String(r[timeCol.name] ?? ""))
    : rows.map((_, i) => i);

  const border = chromeColor("--border");
  const label = chromeColor("--muted-foreground");
  const multi = numericCols.length > 1;

  return {
    grid: {
      left: 8,
      right: 14,
      top: multi ? 30 : 12,
      bottom: 6,
      containLabel: true,
    },
    tooltip: { trigger: "axis" },
    legend: multi
      ? { textStyle: { color: label }, type: "scroll", top: 0 }
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
      axisLine: { lineStyle: { color: border } },
      splitLine: { lineStyle: { color: border, opacity: 0.4 } },
      axisLabel: { color: label },
    },
    series: numericCols.map((col, index) => {
      const color = seriesColor({ value: col.name }, index);
      return {
        type: "line",
        name: col.name,
        showSymbol: false,
        smooth: true,
        lineStyle: { color, width: 2 },
        itemStyle: { color },
        data: rows.map((r) => {
          const v = r[col.name];
          return typeof v === "number" ? v : v == null ? null : Number(v);
        }),
      };
    }),
  };
}

// Whether a result has anything chartable: at least one numeric column and at
// least one row. The Chart tab uses this to show an honest empty state instead
// of a blank canvas.
export function isChartable(result: QueryResponse): boolean {
  return (
    result.rows.length > 0 &&
    result.columns.some((c) => NUMERIC.has(c.type))
  );
}
