import { describe, it, expect } from "vitest";
import { nodeStyleAttrs } from "./node-style.js";

describe("nodeStyleAttrs", () => {
  it("returns an empty bag for missing/empty style", () => {
    expect(nodeStyleAttrs(undefined)).toEqual({});
    expect(nodeStyleAttrs(null)).toEqual({});
    expect(nodeStyleAttrs({})).toEqual({});
  });

  it("maps known decoration tokens to data-sdui-* attributes", () => {
    expect(
      nodeStyleAttrs({
        background: "leaf",
        gradient: "dusk",
        surface: "raised",
        radius: "xl",
        spacing: "lg",
        shadow: "glow",
        text_align: "center",
        font_size: "3xl",
        font_weight: "bold",
        intent: "success",
      }),
    ).toEqual({
      "data-sdui-background": "leaf",
      "data-sdui-gradient": "dusk",
      "data-sdui-surface": "raised",
      "data-sdui-radius": "xl",
      "data-sdui-spacing": "lg",
      "data-sdui-shadow": "glow",
      "data-sdui-text-align": "center",
      "data-sdui-font-size": "3xl",
      "data-sdui-font-weight": "bold",
      "data-sdui-intent": "success",
    });
  });

  it("drops tokens outside the closed sets (no injection)", () => {
    expect(
      nodeStyleAttrs({
        background: "#ff0000",
        gradient: "rainbow",
        radius: "42px",
        font_size: "9001",
      }),
    ).toEqual({});
  });

  it("ignores non-visual NodeStyle machinery", () => {
    expect(
      nodeStyleAttrs({
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        ...({ flex: "1", min_width: "md", className: "x" } as any),
        radius: "md",
      }),
    ).toEqual({ "data-sdui-radius": "md" });
  });
});
