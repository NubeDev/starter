import { describe, it, expect } from "vitest";
import { RenderSlider } from "./render-slider.js";
import { renderHarness } from "./test-utils.js";

describe("RenderSlider", () => {
  it("shows the current value in the label", () => {
    const html = renderHarness(
      <RenderSlider
        node={{ type: "slider", label: "Threshold", page_state_key: "t", min: 0, max: 100 }}
      />,
      { t: 42 },
    );
    expect(html).toContain("Threshold");
    expect(html).toContain("42");
  });
});
