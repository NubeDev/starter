import { describe, expect, it } from "vitest";

import { resolveBound, resolveTimeRange } from "@/store/time/resolve";

// The relative-token resolver is the load-bearing pure function of WS-01:
// every panel query depends on it turning `now-6h`/`now/d` into the same
// absolute instants. Pinned against a fixed `now` so the assertions are
// deterministic regardless of when the suite runs.
describe("resolveBound", () => {
  // 2026-06-09T14:37:25.500Z — but local-time floors below depend on the
  // runner's zone, so we assert relative deltas, not wall-clock literals.
  const now = new Date("2026-06-09T14:37:25.500Z");

  it("resolves `now` to the reference instant", () => {
    expect(resolveBound("now", now).getTime()).toBe(now.getTime());
  });

  it("subtracts fixed-width relative offsets", () => {
    expect(resolveBound("now-6h", now).getTime()).toBe(
      now.getTime() - 6 * 3_600_000,
    );
    expect(resolveBound("now-15m", now).getTime()).toBe(
      now.getTime() - 15 * 60_000,
    );
    expect(resolveBound("now-7d", now).getTime()).toBe(
      now.getTime() - 7 * 86_400_000,
    );
  });

  it("floors `now/d` to the local start of day", () => {
    const d = resolveBound("now/d", now);
    expect(d.getHours()).toBe(0);
    expect(d.getMinutes()).toBe(0);
    expect(d.getSeconds()).toBe(0);
    expect(d.getMilliseconds()).toBe(0);
    // Same calendar day as `now` (in local time), at midnight.
    expect(d.getDate()).toBe(now.getDate());
  });

  it("floors `now/h` to the start of the hour", () => {
    const d = resolveBound("now/h", now);
    expect(d.getMinutes()).toBe(0);
    expect(d.getSeconds()).toBe(0);
    expect(d.getMilliseconds()).toBe(0);
  });

  it("shifts then rounds (`now-1d/d` = yesterday midnight)", () => {
    const today = resolveBound("now/d", now);
    const yesterday = resolveBound("now-1d/d", now);
    expect(today.getTime() - yesterday.getTime()).toBe(86_400_000);
    expect(yesterday.getHours()).toBe(0);
  });

  it("parses absolute ISO instants", () => {
    const iso = "2026-01-02T03:04:05.000Z";
    expect(resolveBound(iso, now).toISOString()).toBe(iso);
  });

  it("throws on an unparseable token", () => {
    expect(() => resolveBound("yesterday-ish", now)).toThrow();
    expect(() => resolveBound("now-3x", now)).toThrow();
  });
});

describe("resolveTimeRange", () => {
  const now = new Date("2026-06-09T14:37:25.500Z");

  it("resolves both bounds against one frozen now", () => {
    const r = resolveTimeRange({ from: "now-6h", to: "now" }, now);
    expect(r.to.getTime()).toBe(now.getTime());
    expect(r.from.getTime()).toBe(now.getTime() - 6 * 3_600_000);
  });
});
