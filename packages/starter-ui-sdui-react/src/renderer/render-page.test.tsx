import { describe, it, expect } from "vitest";
import { RenderPage } from "./render-page.js";
import { renderHarness } from "./test-utils.js";

describe("RenderPage", () => {
  it("renders title and children frame", () => {
    const html = renderHarness(
      <RenderPage node={{ type: "page", title: "Hello", children: [] }} />,
    );
    expect(html).toContain("Hello");
    expect(html).toContain("sdui-page");
  });
});
