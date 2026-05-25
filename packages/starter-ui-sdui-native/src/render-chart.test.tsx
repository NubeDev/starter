import { describe, expect, it } from "vitest";
import { RenderChart } from "./render-chart.js";
import { allByKit, byKit, mount } from "./test-utils.js";

describe("RenderChart", () => {
  it("summarises series/point counts in a Card", () => {
    const root = mount(
      <RenderChart
        node={{
          type: "chart",
          title: "Trend",
          sources: [
            { type: "static", points: [[1, 10], [2, 20]] },
            { type: "static", points: [[1, 5]] },
          ],
        }}
      />,
    );
    const card = byKit(root, "card");
    expect(card).not.toBeNull();
    expect(card?.getAttribute("data-accessibilityrole")).toBe("image");
    expect(card?.getAttribute("data-accessibilitylabel")).toBe("Trend");
    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Trend");
    expect(texts).toContain("2 series · 3 points");
  });

  it("shows 'no data' when sources are empty", () => {
    const root = mount(<RenderChart node={{ type: "chart" }} />);
    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("no data");
  });
});
