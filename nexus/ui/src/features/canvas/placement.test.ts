import { describe, expect, it } from "vitest";

import type { Widget } from "@/data/types";
import { nextSlot } from "@/features/canvas/placement";

const at = (y: number, h: number): Widget => ({
  id: `w${y}`,
  type: "line",
  title: "",
  layout: { x: 0, y, w: 6, h },
  config: { query: { datasourceId: "", sql: "" }, fields: { series: [] } },
});

// A new panel drops below everything else, at column 0. Pinned so adding
// a widget never overlaps an existing one.
describe("nextSlot", () => {
  it("places the first panel at the origin", () => {
    expect(nextSlot([], 6, 4)).toEqual({ x: 0, y: 0, w: 6, h: 4 });
  });

  it("stacks a new panel below the lowest existing one", () => {
    // panels occupy rows 0–3 and 4–8 → next free row is 9.
    const slot = nextSlot([at(0, 4), at(4, 5)], 6, 4);
    expect(slot.y).toBe(9);
    expect(slot.x).toBe(0);
  });
});
