import { describe, expect, it } from "vitest";

import { QUICK_RANGES, rangeLabel } from "@/features/time/quickRanges";
import { resolveBound } from "@/store/time";

// Quick-range integrity: every catalogued range must use tokens the resolver
// accepts (a typo here breaks the picker silently), and the label lookup must
// recognise a catalogued range and fall back legibly otherwise.
describe("quick ranges", () => {
  const now = new Date("2026-06-09T14:37:25.500Z");

  it("every quick range resolves without throwing", () => {
    for (const q of QUICK_RANGES) {
      expect(() => resolveBound(q.range.from, now)).not.toThrow();
      expect(() => resolveBound(q.range.to, now)).not.toThrow();
    }
  });

  it("labels a known range by its catalogue name", () => {
    expect(rangeLabel({ from: "now-6h", to: "now" })).toBe("Last 6 hours");
  });

  it("falls back to echoing an unknown range", () => {
    expect(rangeLabel({ from: "now-3h", to: "now" })).toContain("now-3h");
  });
});
