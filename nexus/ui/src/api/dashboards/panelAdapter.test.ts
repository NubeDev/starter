import { describe, expect, it } from "vitest";

import type { PanelDetail } from "@/api/types";
import type { Widget } from "@/data/types";
import { panelToWidget, widgetToCreatePanel } from "@/api/dashboards/panelAdapter";

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
