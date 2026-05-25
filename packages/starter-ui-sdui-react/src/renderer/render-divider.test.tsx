import { describe, it, expect } from "vitest";
import { RenderDivider } from "./render-divider.js";
import { renderHarness } from "./test-utils.js";

describe("RenderDivider", () => {
  it("emits an element", () => {
    const html = renderHarness(<RenderDivider node={{ type: "divider" }} />);
    expect(html).toContain("sdui-divider");
  });
});
