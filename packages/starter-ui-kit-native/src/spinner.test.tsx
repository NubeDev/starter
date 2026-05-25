import { describe, expect, it } from "vitest";

import { Spinner } from "./spinner.js";
import { a11y, bySlot, mount } from "./test-utils.js";

describe("<Spinner>", () => {
  it("uses ActivityIndicator with accessibilityRole=progressbar", () => {
    const root = mount(<Spinner />);
    const el = bySlot(root, "activity-indicator");
    expect(el).not.toBeNull();
    expect(a11y(el, "accessibilityRole")).toBe("progressbar");
    expect(a11y(el, "accessibilityLabel")).toBe("Loading");
  });

  it("snapshot stable", () => {
    expect(mount(<Spinner size="large" />)).toMatchSnapshot();
  });
});
