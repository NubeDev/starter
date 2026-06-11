import type { EChartsOption } from "echarts";

import type { PanelOptions } from "@/data/types";

// Translates the panel's legend + y-axis options into the ECharts
// `legend`/`yAxis` fragments the cartesian builders (line/area/bar) share.
// Kept in one place so legend placement and axis scale behave identically
// across chart families, and so the builders stay focused on series
// mapping. Pure: options + chrome colours in, ECharts fragments out.

/** Legend fragment honouring show/placement, or undefined to fall back to
 *  the builder's default (shown only when multi-series). `labelColor` is a
 *  resolved chrome colour the caller already read. */
export function legendFragment(
  options: PanelOptions | undefined,
  multi: boolean,
  labelColor: string,
): EChartsOption["legend"] | undefined {
  const legend = options?.legend;
  const show = legend?.show ?? multi;
  if (!show) return undefined;
  const base = { type: "scroll" as const, textStyle: { color: labelColor }, itemWidth: 10, itemHeight: 10 };
  switch (legend?.placement ?? "top") {
    case "right":
      return { ...base, orient: "vertical", right: 0, top: "middle" };
    case "bottom":
      return { ...base, bottom: 0, left: "center" };
    default:
      return { ...base, top: 0, right: 0 };
  }
}

/** Y-axis fragment applying scale (linear/log), soft bounds, and a label.
 *  `borderColor`/`labelColor` are resolved chrome colours from the caller. */
export function yAxisFragment(
  options: PanelOptions | undefined,
  borderColor: string,
  labelColor: string,
): EChartsOption["yAxis"] {
  const axis = options?.yAxis;
  return {
    type: axis?.scale === "log" ? "log" : "value",
    name: axis?.label,
    nameTextStyle: axis?.label ? { color: labelColor } : undefined,
    min: axis?.softMin,
    max: axis?.softMax,
    axisLabel: { color: labelColor },
    splitLine: { lineStyle: { color: borderColor, opacity: 0.4 } },
  };
}
