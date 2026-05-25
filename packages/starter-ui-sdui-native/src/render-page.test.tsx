import { describe, expect, it } from "vitest";
import { RenderPage } from "./render-page.js";
import { byKit, mount } from "./test-utils.js";

describe("RenderPage", () => {
  it("renders a Column with the page title as a header", () => {
    const root = mount(
      <RenderPage node={{ type: "page", title: "Overview", children: [] }} />,
    );
    const col = byKit(root, "column");
    expect(col).not.toBeNull();
    expect(col?.getAttribute("data-accessibilityrole")).toBe("main");
    const text = byKit(root, "text");
    expect(text?.textContent).toBe("Overview");
    expect(text?.getAttribute("data-accessibilityrole")).toBe("header");
  });

  it("renders without a title", () => {
    const root = mount(<RenderPage node={{ type: "page" }} />);
    expect(byKit(root, "column")).not.toBeNull();
    expect(byKit(root, "text")).toBeNull();
  });
});
