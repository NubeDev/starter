import { describe, expect, it } from "vitest";

import type { Widget, WidgetData } from "@/data/types";
import { buildLineOption } from "@/features/widgets/lineOption";

// Typed props in, ECharts option out — a pure contract test (F10). The
// widget that renders this option is a thin wrapper; the mapping logic
// (field mapping → series, axis from `x`, area fill toggle) is what can
// break, so it is what we pin.
const widget: Widget = {
  id: "w",
  type: "line",
  title: "Temp",
  layout: { x: 0, y: 0, w: 6, h: 4 },
  config: {
    query: { datasourceId: "ds", sql: "select ts, a, b from t" },
    fields: {
      x: "ts",
      series: [
        { value: "a", label: "Inlet", unit: "°C" },
        { value: "b" },
      ],
    },
  },
};

const data: WidgetData = {
  points: [
    { ts: "10:00", a: 21, b: 19 },
    { ts: "10:01", a: 22, b: 20 },
  ],
};

describe("buildLineOption", () => {
  it("maps each configured field to a series with its label", () => {
    const opt = buildLineOption(widget, data, { area: false });
    const series = opt.series as Array<{ name?: string; data: unknown[] }>;
    expect(series).toHaveLength(2);
    expect(series[0].name).toBe("Inlet");
    expect(series[1].name).toBe("b");
    expect(series[0].data).toEqual([21, 22]);
  });

  it("derives the category axis from the x column", () => {
    const opt = buildLineOption(widget, data, { area: false });
    const axis = opt.xAxis as { data: unknown[] };
    expect(axis.data).toEqual(["10:00", "10:01"]);
  });

  it("area mode adds an area style; line mode does not", () => {
    const line = buildLineOption(widget, data, { area: false });
    const area = buildLineOption(widget, data, { area: true });
    const lineSeries = (line.series as Array<{ areaStyle?: unknown }>)[0];
    const areaSeries = (area.series as Array<{ areaStyle?: unknown }>)[0];
    expect(lineSeries.areaStyle).toBeUndefined();
    expect(areaSeries.areaStyle).toBeDefined();
  });

  it("renders an empty option when there are no points", () => {
    const opt = buildLineOption(widget, { points: [] }, { area: false });
    const series = opt.series as Array<{ data: unknown[] }>;
    expect(series[0].data).toEqual([]);
  });
});
