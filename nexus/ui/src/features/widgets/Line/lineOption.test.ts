import { describe, expect, it } from "vitest";

import type { Widget, WidgetData } from "@/data/types";
import { buildLineOption } from "@/features/widgets/Line/lineOption";

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

// WS-04: field-config overrides recolour/rename/hide series, and panel
// options drive the legend + y-axis. The builder reads them via
// `resolveField` and `cartesianChrome` so the preview reflects edits.
describe("buildLineOption field config", () => {
  const tempWidget: Widget = {
    ...widget,
    config: {
      ...widget.config,
      fields: { x: "ts", series: [{ value: "temp_in" }, { value: "power" }] },
    },
  };
  const tempData: WidgetData = {
    points: [
      { ts: "1", temp_in: 20, power: 100 },
      { ts: "2", temp_in: 22, power: 110 },
    ],
  };

  it("a /temp/ override applies its colour and display name", () => {
    const opt = buildLineOption(
      {
        ...tempWidget,
        config: {
          ...tempWidget.config,
          fieldConfig: {
            overrides: [
              {
                matcher: { type: "byRegex", value: "temp" },
                display: { color: "0 84% 60%", displayName: "Coolant" },
              },
            ],
          },
        },
      },
      tempData,
      { area: false },
    );
    const series = opt.series as Array<{ name?: string; itemStyle?: { color?: string } }>;
    const temp = series.find((s) => s.name === "Coolant");
    expect(temp).toBeDefined();
    expect(temp?.itemStyle?.color).toBe("hsl(0 84% 60%)");
  });

  it("a hidden override drops the series entirely", () => {
    const opt = buildLineOption(
      {
        ...tempWidget,
        config: {
          ...tempWidget.config,
          fieldConfig: {
            overrides: [{ matcher: { type: "byName", value: "power" }, display: { hidden: true } }],
          },
        },
      },
      tempData,
      { area: false },
    );
    const series = opt.series as Array<{ name?: string }>;
    expect(series).toHaveLength(1);
    expect(series[0].name).toBe("temp_in");
  });

  it("honours an explicit legend placement and a log y-axis", () => {
    const opt = buildLineOption(
      {
        ...tempWidget,
        config: {
          ...tempWidget.config,
          options: { legend: { show: true, placement: "right" }, yAxis: { scale: "log" } },
        },
      },
      tempData,
      { area: false },
    );
    expect(opt.legend).toMatchObject({ orient: "vertical" });
    expect(opt.yAxis).toMatchObject({ type: "log" });
  });
});
