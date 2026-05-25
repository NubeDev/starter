import { describe, expect, it } from "vitest";
import { RenderDateRange } from "./render-date-range.js";
import { Providers } from "./test-wrappers.js";
import { allByKit, mount } from "./test-utils.js";

describe("RenderDateRange", () => {
  it("renders two Inputs hydrated from page_state with a11y labels", () => {
    const root = mount(
      <Providers initialState={{ range: { from: "2026-01-01", to: "2026-01-31" } }}>
        <RenderDateRange
          node={{ type: "date_range", label: "When", page_state_key: "range" }}
        />
      </Providers>,
    );
    const inputs = allByKit(root, "input");
    expect(inputs).toHaveLength(2);
    expect(inputs[0]?.getAttribute("data-value")).toBe("2026-01-01");
    expect(inputs[1]?.getAttribute("data-value")).toBe("2026-01-31");
    expect(inputs[0]?.getAttribute("data-accessibilitylabel")).toBe("When from");
    expect(inputs[1]?.getAttribute("data-accessibilitylabel")).toBe("When to");
  });
});
