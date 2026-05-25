import { describe, expect, it } from "vitest";

import { Switch } from "./switch.js";
import { a11y, bySlot, mount } from "./test-utils.js";

describe("<Switch>", () => {
  it("exposes accessibilityRole=switch with checked state", () => {
    const root = mount(
      <Switch defaultChecked accessibilityLabel="Notifications" />,
    );
    const el = bySlot(root, "pressable");
    expect(a11y(el, "accessibilityRole")).toBe("switch");
    expect(a11y(el, "accessibilityLabel")).toBe("Notifications");
  });

  it("snapshot stable", () => {
    expect(
      mount(<Switch size="sm" accessibilityLabel="Compact" />),
    ).toMatchSnapshot();
  });
});
