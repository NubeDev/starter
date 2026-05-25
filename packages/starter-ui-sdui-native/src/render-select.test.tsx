import { describe, expect, it } from "vitest";
import { RenderSelect } from "./render-select.js";
import { Providers } from "./test-wrappers.js";
import { allByKit, byKit, mount } from "./test-utils.js";

describe("RenderSelect", () => {
  it("renders one SelectItem per option, hydrated from page_state", () => {
    const root = mount(
      <Providers initialState={{ range: "7d" }}>
        <RenderSelect
          node={{
            type: "select",
            label: "Range",
            page_state_key: "range",
            options: [
              { value: "1d", label: "1 day" },
              { value: "7d", label: "7 days" },
            ],
          }}
        />
      </Providers>,
    );
    expect(byKit(root, "select")?.getAttribute("data-value")).toBe("7d");
    expect(
      byKit(root, "select-trigger")?.getAttribute("data-accessibilitylabel"),
    ).toBe("Range");
    expect(allByKit(root, "select-item")).toHaveLength(2);
  });
});
