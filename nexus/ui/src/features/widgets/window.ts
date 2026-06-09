import type { SeriesPoint } from "@/data/types";

// Append a live batch to a bounded sliding window, dropping the oldest
// points once `cap` is exceeded. Pure so the live hook's accumulation is
// testable and the window can never grow without bound (memory safety on
// an unbounded stream).
export function appendWindow(
  current: ReadonlyArray<SeriesPoint>,
  batch: ReadonlyArray<SeriesPoint>,
  cap: number,
): SeriesPoint[] {
  const next = current.concat(batch);
  return next.length > cap ? next.slice(-cap) : next;
}
