// Smoke test: the canonical `disk-overview.json` dashboard from
// `crates/rubix-flows/dashboards/` (also used by web SDUI tests)
// renders end-to-end through the side-effect-registered renderers.
//
// Uses `page`, `row`, `col`, `kpi`, `chart` + `static` slot bindings —
// every renderer touched here is one of the 16 this package ships.

import { describe, expect, it } from "vitest";
import type { UiComponentTree } from "@nube/starter-ui-ir";
import { Render, listRenderers } from "@nube/starter-ui-sdui-react/headless";
import "./index.js"; // side-effect register

import diskOverview from "./test-fixtures/disk-overview.json" with { type: "json" };
import { allByKit, byKit, mount } from "./test-utils.js";

describe("disk-overview smoke", () => {
  it("registers the 16 expected kinds", () => {
    const kinds = listRenderers();
    expect(kinds).toEqual(
      [
        "chart",
        "col",
        "custom",
        "date_range",
        "divider",
        "form",
        "grid",
        "kpi",
        "page",
        "repeat",
        "row",
        "select",
        "slider",
        "tabs",
        "table",
        "toggle",
      ].sort(),
    );
  });

  it("renders the disk-overview fixture end-to-end", () => {
    const tree = diskOverview as unknown as UiComponentTree;
    const root = mount(<Render node={tree.root} />);

    // page title surfaces as a header
    const header = Array.from(root.querySelectorAll('[data-kit="text"]')).find(
      (t) => t.getAttribute("data-accessibilityrole") === "header",
    );
    expect(header?.textContent).toBe("Disk overview");

    // both KPIs render with their values
    const cards = allByKit(root, "card");
    expect(cards.length).toBeGreaterThanOrEqual(3); // 2 KPIs + 1 chart

    const texts = allByKit(root, "text").map((t) => t.textContent);
    expect(texts).toContain("Disk used");
    expect(texts).toContain("42");
    expect(texts).toContain("Disk free");
    expect(texts).toContain("58");

    // chart summary appears
    expect(byKit(root, "chart") ?? null).toBeNull(); // no `chart` kit element — we use Card
    expect(texts.some((t) => t?.includes("series"))).toBe(true);

    // structural containers present
    expect(allByKit(root, "row").length).toBeGreaterThanOrEqual(2);
    expect(allByKit(root, "column").length).toBeGreaterThanOrEqual(3);
  });
});
