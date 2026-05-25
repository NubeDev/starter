import { describe, expect, it } from "vitest";
import { RenderGrid } from "./render-grid.js";
import { allByKit, byKit, mount } from "./test-utils.js";

describe("RenderGrid", () => {
  it("wraps each child in a Box sized to 100/columns %", () => {
    const root = mount(
      <RenderGrid
        node={{
          type: "grid",
          columns: 4,
          children: [
            { type: "divider", id: "a" },
            { type: "divider", id: "b" },
          ],
        }}
      />,
    );
    expect(byKit(root, "row")).not.toBeNull();
    const boxes = allByKit(root, "box");
    expect(boxes).toHaveLength(2);
  });
});
