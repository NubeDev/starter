import { describe, expect, it, vi } from "vitest";

import { Button } from "./button.js";
import { a11y, bySlot, mount } from "./test-utils.js";

describe("<Button>", () => {
  it("renders as a Pressable with accessibilityRole=button", () => {
    const root = mount(<Button>Save</Button>);
    const el = bySlot(root, "pressable");
    expect(el).not.toBeNull();
    expect(a11y(el, "accessibilityRole")).toBe("button");
  });

  it("uses the string child as the implicit accessibilityLabel", () => {
    const root = mount(<Button>Save</Button>);
    expect(a11y(bySlot(root, "pressable"), "accessibilityLabel")).toBe("Save");
  });

  it("prefers an explicit accessibilityLabel", () => {
    const root = mount(
      <Button accessibilityLabel="Persist record">…</Button>,
    );
    expect(a11y(bySlot(root, "pressable"), "accessibilityLabel")).toBe(
      "Persist record",
    );
  });

  it("forwards onPress and reports disabled state", () => {
    const onPress = vi.fn();
    const root = mount(
      <Button onPress={onPress} disabled>
        Save
      </Button>,
    );
    // The mock doesn't dispatch events — we just verify the wiring.
    expect(bySlot(root, "pressable")).toMatchSnapshot();
    expect(onPress).not.toHaveBeenCalled();
  });
});
