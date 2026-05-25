import * as React from "react";
import { describe, expect, it } from "vitest";
import { RenderCustom } from "./render-custom.js";
import { Providers } from "./test-wrappers.js";
import { byKit, mount } from "./test-utils.js";

describe("RenderCustom", () => {
  it("renders a host-supplied custom renderer by renderer_id", () => {
    const Mine: React.ComponentType<{ node: unknown }> = ({ node }) => (
      <div data-kit="mine" data-id={(node as { renderer_id?: string }).renderer_id} />
    );
    const root = mount(
      <Providers customRenderers={{ "rubix.alarm-table": Mine }}>
        <RenderCustom
          node={{ type: "custom", renderer_id: "rubix.alarm-table" }}
        />
      </Providers>,
    );
    expect(byKit(root, "mine")?.getAttribute("data-id")).toBe(
      "rubix.alarm-table",
    );
  });

  it("renders an accessible placeholder when the renderer is missing", () => {
    const root = mount(
      <Providers>
        <RenderCustom
          node={{ type: "custom", renderer_id: "rubix.unknown" }}
        />
      </Providers>,
    );
    const box = byKit(root, "box");
    expect(box?.getAttribute("data-accessibilityrole")).toBe("alert");
    expect(box?.getAttribute("data-accessibilitylabel")).toBe(
      "Missing custom renderer: rubix.unknown",
    );
  });
});
