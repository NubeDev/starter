import { describe, it, expect } from "vitest";
import { RenderForm } from "./render-form.js";
import { renderHarness } from "./test-utils.js";

describe("RenderForm", () => {
  it("renders submit button with label", () => {
    const html = renderHarness(
      <RenderForm node={{ type: "form", submit: { handler: "save", label: "Save" } }} />,
    );
    expect(html).toContain("Save");
    expect(html).toContain("sdui-form");
  });
});
