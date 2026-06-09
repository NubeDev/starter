import { describe, expect, it } from "vitest";

import type { Widget } from "@/data/types";
import {
  applyGridLayout,
  changedWidgets,
  toGridLayout,
} from "@/features/canvas/layout";

const widget = (id: string, x: number, y: number): Widget => ({
  id,
  type: "line",
  title: id,
  layout: { x, y, w: 4, h: 3 },
  config: { query: { datasourceId: "ds", sql: "" }, fields: { series: [{ value: "v" }] } },
});

// The canvas maps between widget layout and react-grid-layout's `Layout`
// items, and only persists when something actually moved. Pinned so a
// no-op layout callback (which RGL fires on mount/resize) never triggers a
// spurious save, and a real drag does.
describe("canvas layout mapping", () => {
  it("projects widgets to grid items, carrying min sizes by type", () => {
    const items = toGridLayout([widget("a", 0, 0)]);
    expect(items[0]).toMatchObject({ i: "a", x: 0, y: 0, w: 4, h: 3 });
    expect(items[0].minW).toBeGreaterThan(0);
  });

  it("applyGridLayout returns null when nothing moved", () => {
    const widgets = [widget("a", 0, 0), widget("b", 4, 0)];
    const same = toGridLayout(widgets);
    expect(applyGridLayout(widgets, same)).toBeNull();
  });

  it("applyGridLayout returns updated widgets when a panel moved", () => {
    const widgets = [widget("a", 0, 0), widget("b", 4, 0)];
    const moved = toGridLayout(widgets).map((l) =>
      l.i === "b" ? { ...l, x: 8, y: 2 } : l,
    );
    const next = applyGridLayout(widgets, moved);
    expect(next).not.toBeNull();
    expect(next![1].layout).toEqual({ x: 8, y: 2, w: 4, h: 3 });
    // unmoved widget is untouched
    expect(next![0].layout).toEqual({ x: 0, y: 0, w: 4, h: 3 });
  });

  it("changedWidgets returns only the panels that actually moved", () => {
    const widgets = [widget("a", 0, 0), widget("b", 4, 0)];
    const moved = toGridLayout(widgets).map((l) =>
      l.i === "b" ? { ...l, x: 8 } : l,
    );
    const changed = changedWidgets(widgets, moved);
    expect(changed.map((w) => w.id)).toEqual(["b"]);
    expect(changed[0].layout.x).toBe(8);
  });

  it("changedWidgets is empty when nothing moved", () => {
    const widgets = [widget("a", 0, 0)];
    expect(changedWidgets(widgets, toGridLayout(widgets))).toEqual([]);
  });
});
