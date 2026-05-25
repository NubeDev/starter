import { describe, it, expect } from "vitest";
// Side-effect import: registers all built-in renderers so the
// central walker can dispatch `template.type` to `render-divider`.
import "./index.js";
import { RenderRepeat } from "./render-repeat.js";
import { renderHarness } from "./test-utils.js";

describe("RenderRepeat", () => {
  it("renders the template once per item", () => {
    const html = renderHarness(
      <RenderRepeat
        node={{
          type: "repeat",
          items: [1, 2, 3],
          template: { type: "divider" },
        }}
      />,
    );
    const count = (html.match(/sdui-divider/g) ?? []).length;
    expect(count).toBe(3);
  });
});
