import { describe, it, expect } from "vitest";
import { RenderKpi } from "./render-kpi.js";
import { renderHarness } from "./test-utils.js";

describe("RenderKpi", () => {
  it("shows label, value, unit", () => {
    const html = renderHarness(
      <RenderKpi node={{ type: "kpi", label: "Disk", value: 42, unit: "%" }} />,
    );
    expect(html).toContain("Disk");
    expect(html).toContain("42");
    expect(html).toContain("%");
  });
});
