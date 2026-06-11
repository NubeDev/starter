import type { EChartsOption } from "echarts";

import type { Widget, WidgetData } from "@/data/types";
import { chromeColor, seriesColor } from "@/features/widgets/_shared/palette";

// Builds the ECharts option for a pie panel. Pure: same inputs → same
// option, no fetching (F6). A pie reads a single series (the slice value)
// and uses the `x` column as the slice label — one row per slice. Each
// slice takes the next palette ramp slot so it tracks the theme (the
// panel re-renders on dark/light switch to rebuild these). With no rows
// it renders an empty pie rather than a fabricated slice (F0).
export function buildPieOption(
  widget: Widget,
  data: WidgetData,
  opts: { donut?: boolean } = {},
): EChartsOption {
  const { x, series } = widget.config.fields;
  const field = series[0];
  const label = chromeColor("--muted-foreground");

  const slices = field
    ? data.points.map((p, i) => ({
        name: x ? String(p[x] ?? `#${i + 1}`) : `#${i + 1}`,
        value: typeof p[field.value] === "number" ? (p[field.value] as number) : 0,
        itemStyle: { color: seriesColor(field, i) },
      }))
    : [];

  return {
    tooltip: { trigger: "item", formatter: "{b}: {c} ({d}%)" },
    legend: {
      type: "scroll",
      orient: "vertical",
      right: 0,
      top: "middle",
      textStyle: { color: label },
      itemWidth: 10,
      itemHeight: 10,
    },
    series: [
      {
        type: "pie",
        // Donut leaves a hole; a plain pie is a full disc.
        radius: opts.donut ? ["45%", "72%"] : "72%",
        center: ["40%", "50%"],
        avoidLabelOverlap: true,
        label: { show: false },
        labelLine: { show: false },
        data: slices,
      },
    ],
  };
}
