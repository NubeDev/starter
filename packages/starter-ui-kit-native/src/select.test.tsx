import { describe, expect, it } from "vitest";

import { Select, SelectContent, SelectItem, SelectTrigger } from "./select.js";
import { a11y, bySlot, mount } from "./test-utils.js";

describe("<Select>", () => {
  it("trigger ships accessibilityRole=combobox + placeholder label fallback", () => {
    const root = mount(
      <Select>
        <SelectTrigger placeholder="Pick" />
        <SelectContent>
          <SelectItem value="x">X</SelectItem>
        </SelectContent>
      </Select>,
    );
    const el = bySlot(root, "pressable");
    expect(a11y(el, "accessibilityRole")).toBe("combobox");
    expect(a11y(el, "accessibilityLabel")).toBe("Pick");
  });

  it("snapshot stable (closed)", () => {
    expect(
      mount(
        <Select>
          <SelectTrigger placeholder="Pick" />
          <SelectContent>
            <SelectItem value="x">X</SelectItem>
          </SelectContent>
        </Select>,
      ),
    ).toMatchSnapshot();
  });
});
