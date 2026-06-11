import { describe, expect, it } from "vitest";

import type { Widget, WidgetData } from "@/data/types";
import { computeStat } from "@/features/widgets/Stat/statDelta";

const widget: Widget = {
  id: "s",
  type: "stat",
  title: "Throughput",
  layout: { x: 0, y: 0, w: 3, h: 2 },
  config: {
    query: { datasourceId: "ds", sql: "select n from rate" },
    fields: { series: [{ value: "n", unit: "rps" }] },
    decimals: 0,
  },
};

describe("computeStat", () => {
  it("reports the latest value and a percent delta vs the prior point", () => {
    const data: WidgetData = { points: [{ n: 100 }, { n: 125 }] };
    const stat = computeStat(widget, data);
    expect(stat?.value).toBe(125);
    expect(stat?.deltaPct).toBeCloseTo(25);
    expect(stat?.trend).toBe("up");
  });

  it("a fall yields a down trend and negative delta", () => {
    const stat = computeStat(widget, { points: [{ n: 80 }, { n: 60 }] });
    expect(stat?.trend).toBe("down");
    expect(stat?.deltaPct).toBeCloseTo(-25);
  });

  it("a single point has a value but no delta", () => {
    const stat = computeStat(widget, { points: [{ n: 42 }] });
    expect(stat?.value).toBe(42);
    expect(stat?.deltaPct).toBeNull();
    expect(stat?.trend).toBe("flat");
  });

  it("no rows yields null — the widget renders empty, not zero (F0)", () => {
    expect(computeStat(widget, { points: [] })).toBeNull();
  });
});
