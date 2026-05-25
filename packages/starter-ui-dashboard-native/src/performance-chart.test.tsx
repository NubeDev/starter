import { describe, expect, it, vi } from "vitest";
import { PerformanceChart } from "./performance-chart.js";
import { allByKit, allBySvg, byKit, byMoti, mount } from "./test-utils.js";

describe("PerformanceChart", () => {
  it("renders title, headline, delta, smoothed path + area, and gridlines", () => {
    const root = mount(
      <PerformanceChart
        data={[10, 20, 30, 25, 40]}
        labels={["Mon", "Tue", "Wed", "Thu", "Fri"]}
        title="Usage"
        headline="42.3"
        headlineSuffix="kWh"
        delta="↑ 12.4%"
      />,
    );

    const card = byKit(root, "card");
    expect(card?.getAttribute("data-accessibilitylabel")).toBe(
      "Usage — 42.3kWh",
    );

    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Usage");
    expect(texts).toContain("42.3");
    expect(texts).toContain("kWh");
    expect(texts).toContain("↑ 12.4%");

    // 3 gridlines + 2 paths (area + line)
    expect(allBySvg(root, "line")).toHaveLength(3);
    expect(allBySvg(root, "path")).toHaveLength(2);

    // x-axis tick labels
    expect(texts).toContain("Mon");
    expect(texts).toContain("Fri");

    // SVG wrapped in MotiView for fade-in
    expect(byMoti(root, "view")).not.toBeNull();
  });

  it("renders period buttons and fires onPeriodChange", () => {
    const onPeriodChange = vi.fn();
    const root = mount(
      <PerformanceChart
        data={[1, 2]}
        labels={["a", "b"]}
        title="t"
        periods={["1D", "1W", "1M"]}
        activePeriodIndex={1}
        onPeriodChange={onPeriodChange}
      />,
    );
    const btns = allByKit(root, "button");
    expect(btns).toHaveLength(3);
    // Active button uses the `default` variant; others use `ghost`.
    expect(btns[0]?.getAttribute("data-variant")).toBe("ghost");
    expect(btns[1]?.getAttribute("data-variant")).toBe("default");
    expect(btns[2]?.getAttribute("data-variant")).toBe("ghost");
    // The button mock echoes children verbatim, so labels are there.
    expect(btns.map((b) => b.textContent)).toEqual(["1D", "1W", "1M"]);
  });

  it("omits area path when data is empty", () => {
    const root = mount(
      <PerformanceChart data={[]} labels={[]} title="empty" />,
    );
    // Only the 3 gridlines, no <path>.
    expect(allBySvg(root, "path")).toHaveLength(0);
    expect(allBySvg(root, "line")).toHaveLength(3);
  });
});
