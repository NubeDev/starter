import { describe, expect, it } from "vitest";
import { RenderKpi } from "./render-kpi.js";
import { allByKit, byKit, mount } from "./test-utils.js";

describe("RenderKpi", () => {
  it("renders label + value from static source.points", () => {
    const root = mount(
      <RenderKpi
        node={{
          type: "kpi",
          label: "Disk used",
          format: "percent",
          unit_symbol: "%",
          source: { type: "static", points: [[0, 42]] },
        }}
      />,
    );
    const card = byKit(root, "card");
    expect(card).not.toBeNull();
    expect(card?.getAttribute("data-accessibilitylabel")).toBe("Disk used: 42 %");
    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Disk used");
    expect(texts).toContain("42");
    expect(texts).toContain("%");
  });

  it("falls back to em-dash when no value or points", () => {
    const root = mount(<RenderKpi node={{ type: "kpi", label: "X" }} />);
    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("—");
  });
});
