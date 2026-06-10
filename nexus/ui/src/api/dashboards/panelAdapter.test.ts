import { describe, expect, it } from "vitest";

import type { PanelDetail } from "@/api/types";
import type { Widget } from "@/data/types";
import {
  panelToWidget,
  widgetToCreatePanel,
  widgetToUpdatePanel,
} from "@/api/dashboards/panelAdapter";

// The backend `PanelDetail` carries title/sql/datasource_id/viz plus an
// *opaque* `layout` JSON the canvas owns. The adapter maps it to the UI's
// `Widget`, stashing the grid position *and* the field mapping inside
// `layout` (the backend has no `fields` column). Round-tripping is the
// contract under test (F10) — typed inputs, not fabricated data (F0).
const panel: PanelDetail = {
  id: "p1",
  title: "Temp",
  sql: "select ts, v from r",
  datasource_id: "ds1",
  viz: "line",
  layout: {
    x: 2,
    y: 0,
    w: 6,
    h: 4,
    fields: { x: "ts", series: [{ value: "v", label: "Temp" }] },
  },
};

describe("panelToWidget", () => {
  it("maps viz→type, sql/datasource→query, and reads layout+fields", () => {
    const w = panelToWidget(panel);
    expect(w.id).toBe("p1");
    expect(w.type).toBe("line");
    expect(w.title).toBe("Temp");
    expect(w.layout).toEqual({ x: 2, y: 0, w: 6, h: 4 });
    expect(w.config.query).toEqual({ datasourceId: "ds1", sql: "select ts, v from r" });
    expect(w.config.fields.series[0].value).toBe("v");
  });

  it("falls back to a table with a default footprint for an unknown viz", () => {
    const w = panelToWidget({ ...panel, viz: "weird", layout: {} });
    expect(w.type).toBe("table");
    expect(w.layout.w).toBeGreaterThan(0);
    expect(w.layout.h).toBeGreaterThan(0);
    // No field mapping in layout → an empty series list, not invented columns.
    expect(w.config.fields.series).toEqual([]);
  });

  it("maps known wire-viz aliases to their widget type", () => {
    expect(panelToWidget({ ...panel, viz: "donut" }).type).toBe("pie");
    expect(panelToWidget({ ...panel, viz: "column" }).type).toBe("bar");
  });

  it("round-trips the full display config (fieldConfig/options/transforms), not just fields", () => {
    // The regression this guards: only `fields` used to be stashed, so every
    // Field/Overrides/Legend/Transforms edit vanished on reload.
    const full: PanelDetail = {
      ...panel,
      layout: {
        x: 0,
        y: 0,
        w: 6,
        h: 4,
        fields: { x: "ts", series: [{ value: "v" }] },
        fieldConfig: {
          defaults: { unit: "kwatth", decimals: 1, thresholds: [{ value: null, color: "1 2% 3%" }] },
          overrides: [{ matcher: { type: "byName", value: "v" }, display: { hidden: true } }],
        },
        options: { legend: { show: true, placement: "right" }, yAxis: { scale: "log" } },
        transforms: [{ kind: "filter", field: "v", op: ">", value: "0" }],
      },
    };
    const w = panelToWidget(full);
    expect(w.config.fieldConfig?.defaults?.unit).toBe("kwatth");
    expect(w.config.fieldConfig?.defaults?.decimals).toBe(1);
    expect(w.config.fieldConfig?.overrides?.[0].display.hidden).toBe(true);
    expect(w.config.options?.legend?.placement).toBe("right");
    expect(w.config.options?.yAxis?.scale).toBe("log");
    expect(w.config.transforms?.[0]).toMatchObject({ kind: "filter", field: "v" });
  });

  it("omits unset display config so the widget stays minimal", () => {
    const w = panelToWidget(panel); // no fieldConfig/options/transforms in layout
    expect(w.config.fieldConfig).toBeUndefined();
    expect(w.config.options).toBeUndefined();
    expect(w.config.transforms).toBeUndefined();
  });
});

describe("full round-trip (widget → create body → widget)", () => {
  it("preserves fieldConfig, options, and transforms through a save+reload", () => {
    const widget: Widget = {
      id: "p1",
      type: "line",
      title: "Full",
      layout: { x: 0, y: 0, w: 6, h: 4 },
      config: {
        query: { datasourceId: "ds", sql: "select t, v from r" },
        fields: { x: "t", series: [{ value: "v" }] },
        fieldConfig: { defaults: { unit: "celsius", decimals: 2 } },
        options: { legend: { show: false }, yAxis: { scale: "log", label: "kW" } },
        transforms: [{ kind: "reduce", field: "v", calc: "avg", as: "avg_v" }],
      },
    };
    const body = widgetToCreatePanel(widget);
    // Simulate the backend echoing `layout` back verbatim into a PanelDetail.
    const reloaded = panelToWidget({
      id: "p1",
      title: body.title,
      sql: body.sql,
      datasource_id: "ds",
      viz: body.viz ?? "table",
      layout: body.layout,
    });
    expect(reloaded.config.fieldConfig).toEqual(widget.config.fieldConfig);
    expect(reloaded.config.options).toEqual(widget.config.options);
    expect(reloaded.config.transforms).toEqual(widget.config.transforms);
    expect(reloaded.config.fields).toEqual(widget.config.fields);
  });
});

describe("widgetToCreatePanel", () => {
  it("packs type/title/query into the create body and stashes layout+fields", () => {
    const widget: Widget = {
      id: "ignored",
      type: "gauge",
      title: "Load",
      layout: { x: 0, y: 0, w: 3, h: 3 },
      config: {
        query: { datasourceId: "ds2", sql: "select v" },
        fields: { series: [{ value: "v" }] },
      },
    };
    const body = widgetToCreatePanel(widget);
    expect(body.viz).toBe("gauge");
    expect(body.title).toBe("Load");
    expect(body.sql).toBe("select v");
    expect(body.datasource_id).toBe("ds2");
    expect(body.layout).toMatchObject({
      x: 0,
      y: 0,
      w: 3,
      h: 3,
      fields: { series: [{ value: "v" }] },
    });
  });
});

describe("widgetToUpdatePanel", () => {
  it("packs the full config (title/sql/datasource/viz) and re-stashes layout+fields", () => {
    const widget: Widget = {
      id: "p1",
      type: "bar",
      title: "Renamed",
      layout: { x: 1, y: 2, w: 6, h: 4 },
      config: {
        query: { datasourceId: "ds9", sql: "select x, v from t" },
        fields: { x: "x", series: [{ value: "v", label: "V" }] },
      },
    };
    const body = widgetToUpdatePanel(widget);
    expect(body.viz).toBe("bar");
    expect(body.title).toBe("Renamed");
    expect(body.sql).toBe("select x, v from t");
    expect(body.datasource_id).toBe("ds9");
    // The edited field mapping rides in the opaque layout alongside the
    // (unchanged) grid position.
    expect(body.layout).toMatchObject({
      x: 1,
      y: 2,
      w: 6,
      h: 4,
      fields: { x: "x", series: [{ value: "v", label: "V" }] },
    });
  });
});
