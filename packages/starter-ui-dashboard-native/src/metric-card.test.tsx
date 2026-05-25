import { describe, expect, it } from "vitest";
import { MetricCard } from "./metric-card.js";
import { allByKit, allBySvg, byKit, byMoti, mount } from "./test-utils.js";

describe("MetricCard", () => {
  it("renders label, animated value, prefix/suffix, and sparkline", () => {
    const root = mount(
      <MetricCard
        label="Energy"
        value={1234}
        prefix="$"
        suffix="kWh"
        spark={[1, 2, 3, 4, 3, 5]}
        accent="#4ade80"
      />,
    );

    const card = byKit(root, "card");
    expect(card).not.toBeNull();
    // a11y label combines prefix/value/suffix
    expect(card?.getAttribute("data-accessibilitylabel")).toBe("Energy: $1,234 kWh");

    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Energy");
    expect(texts).toContain("$");
    expect(texts).toContain("1,234");
    expect(texts).toContain("kWh");

    // sparkline rendered via react-native-svg
    expect(allBySvg(root, "polyline")).toHaveLength(1);
    expect(allBySvg(root, "polygon")).toHaveLength(1);

    // mount uses MotiView (animation pass-through)
    expect(byMoti(root, "view")).not.toBeNull();
  });

  it("renders delta as a Badge with the right variant", () => {
    const root = mount(<MetricCard label="X" value={1} delta={-3.2} />);
    const badge = byKit(root, "badge");
    expect(badge).not.toBeNull();
    expect(badge?.getAttribute("data-variant")).toBe("destructive");
    expect(badge?.textContent).toBe("↓ 3.2%");
  });

  it("omits sparkline when spark is empty", () => {
    const root = mount(<MetricCard label="X" value={1} spark={[]} />);
    expect(allBySvg(root, "svg")).toHaveLength(0);
  });
});
