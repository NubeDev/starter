import { describe, expect, it } from "vitest";

import { intervalSecs } from "@/store/time/interval";

const range = (fromIso: string, toIso: string) => ({
  from: new Date(fromIso),
  to: new Date(toIso),
});

// `$__interval` auto-calculation: a window divided by the point target,
// snapped up to a "nice" bucket. The exact step matters for chart density,
// so pin a few representative windows.
describe("intervalSecs", () => {
  it("buckets a 6h window for ~200 points to a readable step", () => {
    // 6h / 200 = 108s -> nice step is 120s (2m).
    const i = intervalSecs(range("2026-06-09T00:00:00Z", "2026-06-09T06:00:00Z"));
    expect(i).toBe(120);
  });

  it("buckets a 24h window to a coarser step", () => {
    // 24h / 200 = 432s -> nice step 600s (10m).
    const i = intervalSecs(range("2026-06-09T00:00:00Z", "2026-06-10T00:00:00Z"));
    expect(i).toBe(600);
  });

  it("never returns less than 1s", () => {
    const i = intervalSecs(range("2026-06-09T00:00:00Z", "2026-06-09T00:00:00Z"));
    expect(i).toBeGreaterThanOrEqual(1);
  });

  it("honours a custom point target", () => {
    // 1h / 10 = 360s -> nice step 600s (10m).
    const i = intervalSecs(range("2026-06-09T00:00:00Z", "2026-06-09T01:00:00Z"), 10);
    expect(i).toBe(600);
  });
});
