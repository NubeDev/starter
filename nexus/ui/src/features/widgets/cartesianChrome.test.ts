import { describe, expect, it } from "vitest";

import type { PanelOptions } from "@/data/types";
import { legendFragment, yAxisFragment } from "@/features/widgets/cartesianChrome";

// The shared cartesian chrome: legend show/placement and y-axis
// scale/bounds/label, translated into the ECharts fragments the
// line/area/bar builders embed. Pure (F10).
describe("legendFragment", () => {
  it("defaults to shown only when multi-series", () => {
    expect(legendFragment(undefined, false, "#fff")).toBeUndefined();
    expect(legendFragment(undefined, true, "#fff")).toMatchObject({ top: 0 });
  });

  it("an explicit show wins over the multi-series default", () => {
    const opts: PanelOptions = { legend: { show: true } };
    expect(legendFragment(opts, false, "#fff")).toBeDefined();
    expect(legendFragment({ legend: { show: false } }, true, "#fff")).toBeUndefined();
  });

  it("places the legend per the option", () => {
    expect(legendFragment({ legend: { show: true, placement: "right" } }, false, "#fff")).toMatchObject({
      orient: "vertical",
      right: 0,
    });
    expect(legendFragment({ legend: { show: true, placement: "bottom" } }, false, "#fff")).toMatchObject({
      bottom: 0,
    });
  });
});

describe("yAxisFragment", () => {
  it("defaults to a linear value axis", () => {
    expect(yAxisFragment(undefined, "#222", "#fff")).toMatchObject({ type: "value" });
  });

  it("applies log scale, soft bounds, and a label", () => {
    const opts: PanelOptions = { yAxis: { scale: "log", softMin: 1, softMax: 100, label: "kW" } };
    expect(yAxisFragment(opts, "#222", "#fff")).toMatchObject({
      type: "log",
      min: 1,
      max: 100,
      name: "kW",
    });
  });
});
