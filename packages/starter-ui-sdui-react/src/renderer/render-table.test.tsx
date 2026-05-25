import { describe, it, expect } from "vitest";
import { RenderTable } from "./render-table.js";
import { renderHarness } from "./test-utils.js";

describe("RenderTable", () => {
  it("renders header + row cells", () => {
    const html = renderHarness(
      <RenderTable
        node={{
          type: "table",
          columns: [{ key: "name", label: "Name" }, { key: "age" }],
          rows: [{ id: "r1", name: "Ada", age: 36 }],
        }}
      />,
    );
    expect(html).toContain("Name");
    expect(html).toContain("Ada");
    expect(html).toContain("36");
  });
});
