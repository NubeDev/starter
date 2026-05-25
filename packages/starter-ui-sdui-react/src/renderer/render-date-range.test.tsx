import { describe, it, expect } from "vitest";
import { RenderDateRange } from "./render-date-range.js";
import { renderHarness } from "./test-utils.js";

describe("RenderDateRange", () => {
  it("renders both inputs prefilled from page-state", () => {
    const html = renderHarness(
      <RenderDateRange
        node={{ type: "date_range", label: "When", page_state_key: "r" }}
      />,
      { r: { from: "2026-01-01", to: "2026-01-31" } },
    );
    expect(html).toContain("2026-01-01");
    expect(html).toContain("2026-01-31");
  });
});
