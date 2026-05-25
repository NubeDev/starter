import { describe, expect, it } from "vitest";

import { Badge } from "./badge.js";
import { a11y, bySlot, mount } from "./test-utils.js";

describe("<Badge>", () => {
  it("renders as a View with accessibilityRole=text", () => {
    const root = mount(<Badge>v1</Badge>);
    const el = bySlot(root, "view");
    expect(el).not.toBeNull();
    expect(a11y(el, "accessibilityRole")).toBe("text");
    expect(a11y(el, "accessibilityLabel")).toBe("v1");
  });

  it("snapshot stable across variants", () => {
    expect(mount(<Badge variant="destructive">err</Badge>)).toMatchSnapshot();
  });
});
