import type { SeriesPoint } from "@/data/types";

// Reorder each row's columns to follow `order`; columns not listed keep
// their original relative order after the listed ones, and listed columns
// absent from a row are skipped. Object key order is what table renderers
// and `Object.entries` walks honour, so this controls column display
// order without dropping data. Pure (F6).
export function applyOrganize(
  rows: ReadonlyArray<SeriesPoint>,
  order: ReadonlyArray<string>,
): SeriesPoint[] {
  if (order.length === 0) return [...rows];
  return rows.map((row) => {
    const next: SeriesPoint = {};
    for (const key of order) {
      if (key in row) next[key] = row[key];
    }
    for (const key of Object.keys(row)) {
      if (!(key in next)) next[key] = row[key];
    }
    return next;
  });
}
