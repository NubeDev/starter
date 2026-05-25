import { describe, it, expect } from "vitest";
import { RenderCustom } from "./render-custom.js";
import { renderHarness } from "./test-utils.js";

describe("RenderCustom", () => {
  it("falls back to placeholder when renderer is missing", () => {
    const html = renderHarness(
      <RenderCustom node={{ type: "custom", renderer_id: "x.unknown" }} />,
    );
    expect(html).toContain("Missing custom renderer");
    expect(html).toContain("x.unknown");
  });
});
