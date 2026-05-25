import { describe, expect, it } from "vitest";
import { RenderCol } from "./render-col.js";
import { byKit, mount } from "./test-utils.js";

describe("RenderCol", () => {
  it("maps span to flex factor (default 12)", () => {
    const root = mount(<RenderCol node={{ type: "col" }} />);
    expect(byKit(root, "column")?.getAttribute("data-flex")).toBe("12");
  });

  it("clamps span into [1,12]", () => {
    const a = mount(<RenderCol node={{ type: "col", span: 25 }} />);
    expect(byKit(a, "column")?.getAttribute("data-flex")).toBe("12");
    const b = mount(<RenderCol node={{ type: "col", span: 0 }} />);
    expect(byKit(b, "column")?.getAttribute("data-flex")).toBe("1");
    const c = mount(<RenderCol node={{ type: "col", span: 4 }} />);
    expect(byKit(c, "column")?.getAttribute("data-flex")).toBe("4");
  });
});
