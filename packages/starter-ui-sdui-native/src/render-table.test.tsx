import { describe, expect, it } from "vitest";
import { RenderTable } from "./render-table.js";
import { allByKit, byKit, mount } from "./test-utils.js";

describe("RenderTable", () => {
  it("renders header + one Row per data row inside a horizontal ScrollArea", () => {
    const root = mount(
      <RenderTable
        node={{
          type: "table",
          columns: [{ key: "name", label: "Name" }, { key: "age" }],
          rows: [
            { name: "Ada", age: 36 },
            { name: "Linus", age: 54 },
          ],
        }}
      />,
    );
    const sa = byKit(root, "scroll-area");
    expect(sa).not.toBeNull();
    expect(sa?.getAttribute("data-horizontal")).toBe("true");
    // 1 header Row + 2 data Rows
    expect(allByKit(root, "row")).toHaveLength(3);
    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Name");
    expect(texts).toContain("age");
    expect(texts).toContain("Ada");
    expect(texts).toContain("54");
  });
});
