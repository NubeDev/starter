import { describe, it, expect } from "vitest";
import { RenderSelect } from "./render-select.js";
import { renderHarness } from "./test-utils.js";

describe("RenderSelect", () => {
  it("renders the label", () => {
    const html = renderHarness(
      <RenderSelect
        node={{
          type: "select",
          label: "Pick one",
          page_state_key: "k",
          options: [{ value: "a", label: "A" }],
        }}
      />,
    );
    expect(html).toContain("Pick one");
  });
});
