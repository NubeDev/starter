import { describe, expect, it } from "vitest";
import { RadialProgress } from "./radial-progress.js";
import { allByKit, allBySvg, byKit, mount } from "./test-utils.js";

describe("RadialProgress", () => {
  it("renders two circles (track + arc) and the percentage", () => {
    const root = mount(
      <RadialProgress value={42} label="Battery" subLabel="12h remaining" />,
    );
    const card = byKit(root, "card");
    expect(card?.getAttribute("data-accessibilityrole")).toBe("progressbar");
    expect(card?.getAttribute("data-accessibilitylabel")).toBe(
      "Battery: 42% — 12h remaining",
    );

    expect(allBySvg(root, "circle")).toHaveLength(2);

    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Battery");
    expect(texts).toContain("42");
    expect(texts).toContain("%");
    expect(texts).toContain("12h remaining");
  });

  it("clamps value to [0,100]", () => {
    const overshoot = mount(<RadialProgress value={150} label="x" />);
    expect(
      allByKit(overshoot, "text").some((t) => t.textContent === "100"),
    ).toBe(true);

    const undershoot = mount(<RadialProgress value={-20} label="x" />);
    expect(
      allByKit(undershoot, "text").some((t) => t.textContent === "0"),
    ).toBe(true);
  });
});
