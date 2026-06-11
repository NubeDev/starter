import { describe, expect, it } from "vitest";

import type { SeriesPoint } from "@/data/types";
import { appendWindow } from "@/features/widgets/_shared/window";

const pts = (...ns: number[]): SeriesPoint[] => ns.map((n) => ({ v: n }));

// A live panel keeps a bounded sliding window: new batches append, the
// oldest points fall off the front once the cap is hit. Pinned so the
// live feed can't grow unbounded and leak memory.
describe("appendWindow", () => {
  it("appends a batch when under the cap", () => {
    expect(appendWindow(pts(1, 2), pts(3), 10)).toEqual(pts(1, 2, 3));
  });

  it("drops the oldest points once the cap is exceeded", () => {
    expect(appendWindow(pts(1, 2, 3), pts(4, 5), 3)).toEqual(pts(3, 4, 5));
  });

  it("keeps only the last `cap` when a single batch overflows", () => {
    expect(appendWindow([], pts(1, 2, 3, 4, 5), 3)).toEqual(pts(3, 4, 5));
  });

  it("returns the prior window unchanged for an empty batch", () => {
    expect(appendWindow(pts(1, 2), [], 10)).toEqual(pts(1, 2));
  });
});
