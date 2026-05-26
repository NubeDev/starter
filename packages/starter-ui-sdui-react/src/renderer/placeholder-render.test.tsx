import { describe, it, expect } from "vitest";
import { PlaceholderRender } from "../headless/placeholder-render.js";
import { renderHarness } from "./test-utils.js";
// Side-effect: registers web renderers including kpi_grid.
import "./index.js";

describe("PlaceholderRender", () => {
  it("fills a kpi with sample value + unit when none authored", () => {
    const html = renderHarness(<PlaceholderRender node={{ type: "kpi" }} />);
    expect(html).toContain("Sample KPI");
    expect(html).toContain("123.4");
    expect(html).toContain("kWh");
  });

  it("preserves author-supplied kpi fields", () => {
    const html = renderHarness(
      <PlaceholderRender node={{ type: "kpi", label: "Disk", value: 42, unit_symbol: "%" }} />,
    );
    expect(html).toContain("Disk");
    expect(html).toContain("42");
    expect(html).toContain("%");
  });

  it("fills a chart with a sample series", () => {
    const html = renderHarness(<PlaceholderRender node={{ type: "chart" }} />);
    expect(html).toContain("Sample chart");
    expect(html).toContain("data-sdui-chart-series-count=\"1\"");
  });

  it("fills a table with sample rows + columns", () => {
    const html = renderHarness(<PlaceholderRender node={{ type: "table" }} />);
    expect(html).toContain("Item one");
    expect(html).toContain("Item three");
  });

  it("fills a kpi_grid with sample tiles", () => {
    const html = renderHarness(<PlaceholderRender node={{ type: "kpi_grid" }} />);
    expect(html).toContain("Active");
    expect(html).toContain("Pending");
    expect(html).toContain("Errors");
    expect(html).toContain("data-sdui-kpi-grid-cols=\"3\"");
  });

  it("fills a repeat with a sample template + 3 rows", () => {
    const html = renderHarness(<PlaceholderRender node={{ type: "repeat" }} />);
    // The synthetic template is a kpi labelled "Sample"; three rows
    // means three occurrences.
    const occurrences = html.split("Sample").length - 1;
    expect(occurrences).toBeGreaterThanOrEqual(3);
  });

  it("fills a form with sample children and drops submit", () => {
    const html = renderHarness(<PlaceholderRender node={{ type: "form" }} />);
    // The form's children are text_field placeholders, which have
    // no registered renderer — the dangling-variant tile fires.
    // What matters is that the form frame renders without trying
    // to dispatch a submit handler.
    expect(html).toContain("sdui-form");
    expect(html).not.toContain("Submit");
  });

  it("shows the dangling tile for variants with no filler and no renderer", () => {
    const html = renderHarness(
      <PlaceholderRender node={{ type: "definitely_not_a_real_variant" }} />,
    );
    expect(html).toContain("no placeholder yet");
    expect(html).toContain("definitely_not_a_real_variant");
  });

  it("dispatches through the live renderer when no filler exists but renderer does", () => {
    // `divider` has a live renderer but no placeholder filler —
    // it should still render the live divider, not the missing tile.
    const html = renderHarness(<PlaceholderRender node={{ type: "divider" }} />);
    expect(html).not.toContain("no placeholder yet");
  });
});
