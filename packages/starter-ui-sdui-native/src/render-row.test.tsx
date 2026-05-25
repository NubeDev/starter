import { describe, expect, it } from "vitest";
import { RenderRow } from "./render-row.js";
import { byKit, mount } from "./test-utils.js";

describe("RenderRow", () => {
  it("renders a kit Row", () => {
    const root = mount(<RenderRow node={{ type: "row", id: "r1" }} />);
    const row = byKit(root, "row");
    expect(row).not.toBeNull();
    expect(row?.getAttribute("data-testid")).toBe("r1");
  });
});
