import type { Layout } from "react-grid-layout";

import type { Widget, WidgetType } from "@/data/types";

// Minimum grid footprint per panel type — a gauge needs height, a table
// needs width. Enforced by react-grid-layout during resize.
const MIN_SIZE: Record<WidgetType, { minW: number; minH: number }> = {
  stat: { minW: 2, minH: 2 },
  gauge: { minW: 2, minH: 3 },
  line: { minW: 3, minH: 3 },
  area: { minW: 3, minH: 3 },
  status: { minW: 3, minH: 3 },
  table: { minW: 4, minH: 4 },
};

// Project widgets onto react-grid-layout items. The widget id is the grid
// key, so layout changes map back unambiguously.
export function toGridLayout(widgets: ReadonlyArray<Widget>): Layout[] {
  return widgets.map((w) => ({
    i: w.id,
    x: w.layout.x,
    y: w.layout.y,
    w: w.layout.w,
    h: w.layout.h,
    ...MIN_SIZE[w.type],
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
