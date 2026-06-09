import { describe, expect, it } from "vitest";

import type {
  Dashboard,
  PanelQuery,
  Widget,
  WidgetConfig,
} from "@/data/types";

// The data model is the stable contract that survives every layer swap
// (F7). These are typed-construction assertions: they fail to compile if
// the contract loses a field, and assert at runtime that a real panel is
// keyed by a datasource + query + field mapping — never a fake `metric`
// string. Typed literals here are test inputs, not mock data (F0).
describe("data/types contract", () => {
  it("a panel references a datasource, a query, and a field mapping", () => {
    const query: PanelQuery = {
      datasourceId: "ds_timescale_main",
      sql: "select ts, value from readings where sensor = $1 order by ts",
      params: ["temp-aisle-3"],
    };

    const config: WidgetConfig = {
      query,
      fields: { x: "ts", series: [{ value: "value", label: "Temp", unit: "°C" }] },
      thresholds: { warn: 30, crit: 40 },
      decimals: 1,
    };

    const widget: Widget = {
      id: "w_1",
      type: "line",
      title: "Aisle 3 temperature",
      layout: { x: 0, y: 0, w: 6, h: 4 },
      config,
    };

    expect(widget.config.query.datasourceId).toBe("ds_timescale_main");
    expect(widget.config.fields.series[0].value).toBe("value");
    // No `metric` key exists on the config contract anymore.
    expect("metric" in widget.config).toBe(false);
  });

  it("a panel may declare a live stream binding without a chart-lib coupling", () => {
    const widget: Widget = {
      id: "w_live",
      type: "stat",
      title: "Throughput",
      layout: { x: 0, y: 0, w: 3, h: 2 },
      config: {
        query: { datasourceId: "ds_kafka", sql: "select n from rate" },
        fields: { series: [{ value: "n" }] },
        live: { streamId: "stream_throughput" },
      },
    };

    expect(widget.config.live?.streamId).toBe("stream_throughput");
  });

  it("a dashboard owns its widgets and metadata", () => {
    const dashboard: Dashboard = {
      id: "db_1",
      name: "Cold chain",
      slug: "cold-chain",
      icon: "snowflake",
      accent: "152 76% 44%",
      widgets: [],
      updatedAt: "2026-06-09T00:00:00Z",
    };

    expect(dashboard.widgets).toHaveLength(0);
    expect(dashboard.slug).toBe("cold-chain");
  });
});
