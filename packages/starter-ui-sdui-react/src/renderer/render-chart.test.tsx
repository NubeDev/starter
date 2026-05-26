import { describe, it, expect } from "vitest";
import { RenderChart } from "./render-chart.js";
import { renderHarness } from "./test-utils.js";

describe("RenderChart", () => {
  it("renders placeholder with series count", () => {
    const html = renderHarness(
      <RenderChart node={{ type: "chart", title: "Load", series: [{}, {}, {}] }} />,
    );
    expect(html).toContain("Load");
    expect(html).toContain('data-sdui-chart-series-count="0"');
  });
});
