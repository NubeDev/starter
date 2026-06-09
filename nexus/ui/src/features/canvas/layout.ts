import type { Layout } from "react-grid-layout";

import type { Widget } from "@/data/types";
import { WIDGET_CATALOG } from "@/features/widgets/catalog";

// Project widgets onto react-grid-layout items. The widget id is the grid
// key, so layout changes map back unambiguously. The minimum footprint
// per type (a gauge needs height, a table needs width) comes from the
// widget catalog so it can't drift from the renderers/sizes.
export function toGridLayout(widgets: ReadonlyArray<Widget>): Layout[] {
  return widgets.map((w) => ({
    i: w.id,
    x: w.layout.x,
    y: w.layout.y,
    w: w.layout.w,
    h: w.layout.h,
    ...WIDGET_CATALOG[w.type].minSize,
  }));
}

// Fold a grid layout back onto the widgets. Returns the updated widgets
// only if a position/size actually changed — react-grid-layout fires
// `onLayoutChange` on mount and resize observation too, and persisting
// those no-ops would churn the backend. Null means "nothing to save".
export function applyGridLayout(
  widgets: ReadonlyArray<Widget>,
  layout: ReadonlyArray<Layout>,
): Widget[] | null {
  const byId = new Map(layout.map((l) => [l.i, l]));
  let changed = false;
  const next = widgets.map((w) => {
    const l = byId.get(w.id);
    if (!l) return w;
    if (
      l.x !== w.layout.x ||
      l.y !== w.layout.y ||
      l.w !== w.layout.w ||
      l.h !== w.layout.h
    ) {
      changed = true;
      return { ...w, layout: { x: l.x, y: l.y, w: l.w, h: l.h } };
    }
    return w;
  });
  return changed ? next : null;
}

// The subset of widgets whose position/size changed, with their new
// layouts. Used to persist a drag as one PATCH per moved panel — sending
// only what changed, not the whole board.
export function changedWidgets(
  widgets: ReadonlyArray<Widget>,
  layout: ReadonlyArray<Layout>,
): Widget[] {
  const byId = new Map(layout.map((l) => [l.i, l]));
  const moved: Widget[] = [];
  for (const w of widgets) {
    const l = byId.get(w.id);
    if (!l) continue;
    if (
      l.x !== w.layout.x ||
      l.y !== w.layout.y ||
      l.w !== w.layout.w ||
      l.h !== w.layout.h
    ) {
      moved.push({ ...w, layout: { x: l.x, y: l.y, w: l.w, h: l.h } });
    }
  }
  return moved;
}
