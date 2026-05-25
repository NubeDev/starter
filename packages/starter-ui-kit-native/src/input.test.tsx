import { describe, expect, it } from "vitest";

import { Input } from "./input.js";
import { a11y, bySlot, mount } from "./test-utils.js";

describe("<Input>", () => {
  it("forwards accessibilityLabel and editable state to TextInput", () => {
    const root = mount(
      <Input accessibilityLabel="Email" placeholder="you@example.com" />,
    );
    const el = bySlot(root, "textinput");
    expect(el).not.toBeNull();
    expect(a11y(el, "accessibilityLabel")).toBe("Email");
    expect(el?.getAttribute("placeholder")).toBe("you@example.com");
  });

  it("marks editable=false when disabled", () => {
    const root = mount(<Input accessibilityLabel="X" disabled />);
    const el = bySlot(root, "textinput");
    // Mock copies `editable` verbatim — jsdom lower-cases attribute keys.
    expect(el?.getAttribute("editable")).toBe("false");
  });

  it("snapshot stable", () => {
    expect(
      mount(<Input accessibilityLabel="Search" placeholder="search" />),
    ).toMatchSnapshot();
  });
});
