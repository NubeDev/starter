import { describe, expect, it } from "vitest";
import { RenderToggle } from "./render-toggle.js";
import { Providers } from "./test-wrappers.js";
import { byKit, mount } from "./test-utils.js";

describe("RenderToggle", () => {
  it("reflects boolean page_state and labels the switch for a11y", () => {
    const root = mount(
      <Providers initialState={{ live: true }}>
        <RenderToggle
          node={{ type: "toggle", label: "Live updates", page_state_key: "live" }}
        />
      </Providers>,
    );
    const sw = byKit(root, "switch");
    expect(sw?.getAttribute("data-checked")).toBe("true");
    expect(sw?.getAttribute("data-accessibilitylabel")).toBe("Live updates");
  });
});
