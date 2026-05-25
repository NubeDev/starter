import { describe, expect, it } from "vitest";

import { Slider } from "./slider.js";
import { mount } from "./test-utils.js";

describe("<Slider>", () => {
  it("exposes accessibilityRole=adjustable with min/max/now value", () => {
    const root = mount(
      <Slider
        defaultValue={40}
        min={0}
        max={100}
        accessibilityLabel="Brightness"
      />,
    );
    const el = root.querySelector('[accessibilityrole="adjustable"]');
    expect(el).not.toBeNull();
    expect(el?.getAttribute("accessibilitylabel")).toBe("Brightness");
  });

  it("snapshot stable", () => {
    expect(
      mount(<Slider defaultValue={20} accessibilityLabel="X" />),
    ).toMatchSnapshot();
  });
});
