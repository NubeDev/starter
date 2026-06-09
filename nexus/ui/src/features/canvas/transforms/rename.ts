import type { SeriesPoint } from "@/data/types";

// Rename a field across every row: copy `from` into `to`, drop `from`.
// Pure — returns new rows, never mutates the input (F6). A no-op when
// `from` and `to` are equal or `from` is absent.
export function applyRename(
  rows: ReadonlyArray<SeriesPoint>,
  from: string,
  to: string,
): SeriesPoint[] {
  if (!from || !to || from === to) return [...rows];
  return rows.map((row) => {
    if (!(from in row)) return { ...row };
    const next: SeriesPoint = { ...row, [to]: row[from] };
    delete next[from];
    return next;
  });
}
