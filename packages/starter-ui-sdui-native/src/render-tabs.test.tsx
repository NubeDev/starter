import { describe, expect, it } from "vitest";
import { RenderTabs } from "./render-tabs.js";
import { allByKit, byKit, mount } from "./test-utils.js";

describe("RenderTabs", () => {
  it("renders nothing when tabs are missing", () => {
    const root = mount(<RenderTabs node={{ type: "tabs" }} />);
    expect(byKit(root, "tabs")).toBeNull();
  });

  it("renders one trigger + content per tab", () => {
    const root = mount(
      <RenderTabs
        node={{
          type: "tabs",
          tabs: [
            { id: "a", label: "A", children: [] },
            { id: "b", label: "B", children: [] },
          ],
        }}
      />,
    );
    expect(byKit(root, "tabs")?.getAttribute("data-defaultvalue")).toBe("a");
    expect(allByKit(root, "tabs-trigger")).toHaveLength(2);
    expect(allByKit(root, "tabs-content")).toHaveLength(2);
  });
});
