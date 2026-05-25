import { describe, it, expect } from "vitest";
import { RenderToggle } from "./render-toggle.js";
import { renderHarness } from "./test-utils.js";

describe("RenderToggle", () => {
  it("renders the label", () => {
    const html = renderHarness(
      <RenderToggle
        node={{ type: "toggle", label: "Enabled", page_state_key: "e" }}
      />,
      { e: true },
    );
    expect(html).toContain("Enabled");
  });
});
