import { describe, expect, it } from "vitest";
import { RenderSlider } from "./render-slider.js";
import { Providers } from "./test-wrappers.js";
import { byKit, mount } from "./test-utils.js";

describe("RenderSlider", () => {
  it("binds value to page_state with min/max/step", () => {
    const root = mount(
      <Providers initialState={{ threshold: 30 }}>
        <RenderSlider
          node={{
            type: "slider",
            label: "Threshold",
            page_state_key: "threshold",
            min: 0,
            max: 100,
            step: 5,
          }}
        />
      </Providers>,
    );
    const s = byKit(root, "slider");
    expect(s?.getAttribute("data-value")).toBe("30");
    expect(s?.getAttribute("data-min")).toBe("0");
    expect(s?.getAttribute("data-max")).toBe("100");
    expect(s?.getAttribute("data-step")).toBe("5");
    expect(s?.getAttribute("data-accessibilitylabel")).toBe("Threshold");
  });

  it("falls back to min when page_state is empty", () => {
    const root = mount(
      <Providers>
        <RenderSlider
          node={{ type: "slider", page_state_key: "x", min: 10, max: 20 }}
        />
      </Providers>,
    );
    expect(byKit(root, "slider")?.getAttribute("data-value")).toBe("10");
  });
});
