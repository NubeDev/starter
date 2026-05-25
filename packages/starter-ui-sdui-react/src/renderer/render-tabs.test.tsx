import { describe, it, expect } from "vitest";
import { RenderTabs } from "./render-tabs.js";
import { renderHarness } from "./test-utils.js";

describe("RenderTabs", () => {
  it("renders one trigger per tab", () => {
    const html = renderHarness(
      <RenderTabs
        node={{
          type: "tabs",
          tabs: [
            { id: "a", label: "Alpha", children: [] },
            { id: "b", label: "Beta", children: [] },
          ],
        }}
      />,
    );
    expect(html).toContain("Alpha");
    expect(html).toContain("Beta");
  });
});
