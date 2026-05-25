import { describe, expect, it } from "vitest";
import { RenderDivider } from "./render-divider.js";
import { byKit, mount } from "./test-utils.js";

describe("RenderDivider", () => {
  it("defaults to horizontal", () => {
    const root = mount(<RenderDivider node={{ type: "divider" }} />);
    const d = byKit(root, "divider");
    expect(d?.getAttribute("data-orientation")).toBe("horizontal");
  });

  it("renders vertical when requested", () => {
    const root = mount(
      <RenderDivider node={{ type: "divider", orientation: "vertical" }} />,
    );
    expect(byKit(root, "divider")?.getAttribute("data-orientation")).toBe(
      "vertical",
    );
  });
});
