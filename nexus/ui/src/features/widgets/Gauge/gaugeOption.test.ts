import { describe, expect, it } from "vitest";

import type { Widget, WidgetData } from "@/data/types";
import { buildGaugeOption } from "@/features/widgets/Gauge/gaugeOption";

const widget = (min: number, max: number): Widget => ({
  id: "g",
  type: "gauge",
  title: "Load",
  layout: { x: 0, y: 0, w: 3, h: 3 },
  config: {
    query: { datasourceId: "ds", sql: "select v from m" },
    fields: { series: [{ value: "v", unit: "%" }] },
    thresholds: { warn: 70, crit: 90 },
    min,
    max,
  },
});

const data = (v: number): WidgetData => ({ points: [{ v }] });

describe("buildGaugeOption", () => {
  it("places the value within the configured min/max range", () => {
    const opt = buildGaugeOption(widget(0, 100), data(42));
    const series = (opt.series as Array<{ min: number; max: number; data: Array<{ value: number }> }>)[0];
    expect(series.min).toBe(0);
    expect(series.max).toBe(100);
    expect(series.data[0].value).toBe(42);
  });

  it("colours the arc by threshold state", () => {
    const ok = buildGaugeOption(widget(0, 100), data(40));
    const crit = buildGaugeOption(widget(0, 100), data(95));
    const okColor = readProgressColor(ok);
    const critColor = readProgressColor(crit);
    expect(okColor).not.toBe(critColor);
  });

  it("renders no value when there are no rows (no fabricated zero)", () => {
    const opt = buildGaugeOption(widget(0, 100), { points: [] });
    const series = (opt.series as Array<{ data: Array<{ value: number }> }>)[0];
    // A constant-length data array keeps ECharts' animation from
    // interpolating against a missing element (the empty-array → populated
    // transition throws); the single value is NaN, which the detail
    // formatter renders blank — no fabricated reading (F0).
    expect(series.data).toHaveLength(1);
    expect(Number.isNaN(series.data[0].value)).toBe(true);
  });
});

function readProgressColor(opt: ReturnType<typeof buildGaugeOption>): unknown {
  const series = (opt.series as Array<{ itemStyle?: { color?: unknown } }>)[0];
  return series.itemStyle?.color;
}
