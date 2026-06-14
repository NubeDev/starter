import { describe, expect, it } from "vitest";

import type { Widget, WidgetData } from "@/data/types";
import { buildBarOption } from "@/features/widgets/Bar/barOption";
import { buildPieOption } from "@/features/widgets/Pie/pieOption";
import { buildScatterOption } from "@/features/widgets/Scatter/scatterOption";
import { buildHeatmapOption } from "@/features/widgets/Heatmap/heatmapOption";

// Typed props in, ECharts option out — pure contract tests (F10) for the
// chart-family builders added alongside the existing line/area/gauge. The
// panels that render these are thin wrappers; the mapping logic is what
// can break, so it is what we pin.
function widget(overrides: Partial<Widget> = {}): Widget {
  return {
    id: "w",
    type: "bar",
    title: "Panel",
    layout: { x: 0, y: 0, w: 6, h: 4 },
    config: {
      query: { datasourceId: "ds", sql: "select x, a, b from t" },
      fields: {
        x: "x",
        series: [{ value: "a", label: "A" }, { value: "b" }],
      },
    },
    ...overrides,
  };
}

describe("buildBarOption", () => {
  const data: WidgetData = {
    points: [
      { x: "Mon", a: 3, b: 5 },
      { x: "Tue", a: 4, b: 6 },
    ],
  };

  it("maps each field to a bar series and the x column to the category axis", () => {
    const opt = buildBarOption(widget(), data);
    const series = opt.series as Array<{ type: string; name?: string; data: unknown[] }>;
    expect(series).toHaveLength(2);
    expect(series[0].type).toBe("bar");
    expect(series[0].name).toBe("A");
    expect(series[0].data).toEqual([3, 4]);
    expect((opt.xAxis as { data: unknown[] }).data).toEqual(["Mon", "Tue"]);
  });

  it("renders empty bar data with no points", () => {
    const opt = buildBarOption(widget(), { points: [] });
    expect((opt.series as Array<{ data: unknown[] }>)[0].data).toEqual([]);
  });
});

describe("buildPieOption", () => {
  const w = widget({ type: "pie" });
  const data: WidgetData = {
    points: [
      { x: "Solar", a: 60 },
      { x: "Grid", a: 40 },
    ],
  };

  it("builds one slice per row, labelled by the x column, valued by the first series", () => {
    const opt = buildPieOption(w, data);
    const slices = (opt.series as Array<{ data: Array<{ name: string; value: number }> }>)[0].data;
    expect(slices).toHaveLength(2);
    expect(slices[0]).toMatchObject({ name: "Solar", value: 60 });
    expect(slices[1]).toMatchObject({ name: "Grid", value: 40 });
  });

  it("donut mode uses an inner radius", () => {
    const opt = buildPieOption(w, data, { donut: true });
    const radius = (opt.series as Array<{ radius: unknown }>)[0].radius;
    expect(Array.isArray(radius)).toBe(true);
  });

  it("renders no slices with no points", () => {
    const opt = buildPieOption(w, { points: [] });
    expect((opt.series as Array<{ data: unknown[] }>)[0].data).toEqual([]);
  });
});

describe("buildScatterOption", () => {
  const w = widget({ type: "scatter" });

  it("emits [x, y] pairs and drops rows with non-numeric x or y", () => {
    const data: WidgetData = {
      points: [
        { x: 1, a: 10 },
        { x: 2, a: 20 },
        { x: "bad", a: 30 },
        { x: 3, a: null },
      ],
    };
    const opt = buildScatterOption(w, data);
    const series = opt.series as Array<{ type: string; data: number[][] }>;
    expect(series[0].type).toBe("scatter");
    expect(series[0].data).toEqual([
      [1, 10],
      [2, 20],
    ]);
  });
});

describe("buildHeatmapOption", () => {
  // Heatmap: x = x-axis category, series[0] = y-axis category,
  // series[1] = cell value.
  const w = widget({
    type: "heatmap",
    config: {
      query: { datasourceId: "ds", sql: "select hour, day, v from t" },
      fields: { x: "hour", series: [{ value: "day" }, { value: "v" }] },
    },
  });

  it("builds [xIndex, yIndex, value] cells against distinct category axes", () => {
    const data: WidgetData = {
      points: [
        { hour: "00", day: "Mon", v: 1 },
        { hour: "01", day: "Mon", v: 2 },
        { hour: "00", day: "Tue", v: 3 },
      ],
    };
    const opt = buildHeatmapOption(w, data);
    expect((opt.xAxis as { data: unknown[] }).data).toEqual(["00", "01"]);
    expect((opt.yAxis as { data: unknown[] }).data).toEqual(["Mon", "Tue"]);
    const cells = (opt.series as Array<{ data: number[][] }>)[0].data;
    expect(cells).toEqual([
      [0, 0, 1],
      [1, 0, 2],
      [0, 1, 3],
    ]);
  });

  it("renders no cells when the value series is missing", () => {
    const single = widget({
      type: "heatmap",
      config: {
        query: { datasourceId: "ds", sql: "select hour, day from t" },
        fields: { x: "hour", series: [{ value: "day" }] },
      },
    });
    const opt = buildHeatmapOption(single, {
      points: [{ hour: "00", day: "Mon" }],
    });
    expect((opt.series as Array<{ data: unknown[] }>)[0].data).toEqual([]);
  });
});
