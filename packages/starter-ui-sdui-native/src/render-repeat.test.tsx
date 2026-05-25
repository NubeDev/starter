import { describe, expect, it } from "vitest";
import "./render-divider.js"; // register so the inner template resolves
import { RenderRepeat } from "./render-repeat.js";
import { allByKit, byKit, mount } from "./test-utils.js";

describe("RenderRepeat", () => {
  it("renders nothing without a template", () => {
    const root = mount(
      <RenderRepeat node={{ type: "repeat", items: [1, 2] }} />,
    );
    expect(byKit(root, "column")).toBeNull();
  });

  it("renders one template instance per item", () => {
    const root = mount(
      <RenderRepeat
        node={{
          type: "repeat",
          items: [1, 2, 3],
          template: { type: "divider" },
        }}
      />,
    );
    expect(allByKit(root, "divider")).toHaveLength(3);
  });
});
