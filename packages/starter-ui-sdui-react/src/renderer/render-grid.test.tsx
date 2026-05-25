import { describe, it, expect } from "vitest";
import { RenderGrid } from "./render-grid.js";
import { renderHarness } from "./test-utils.js";

describe("RenderGrid", () => {
  it("emits grid template with N columns", () => {
    const html = renderHarness(
      <RenderGrid node={{ type: "grid", columns: 4 }} />,
    );
    expect(html).toContain("repeat(4");
  });
});
