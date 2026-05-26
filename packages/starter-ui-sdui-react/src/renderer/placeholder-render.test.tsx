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

  // --- per-variant visual placeholders (stage 3) --------------------------
  //
  // One snapshot-ish assertion per variant that previously fell through to
  // the dangling tile. Each placeholder mirrors the live renderer's visual
  // idiom; we just assert the variant marker + a couple of identifying
  // pieces so the test catches "filler disappeared" without locking us in
  // to exact markup.

  const NEW_PLACEHOLDERS: ReadonlyArray<{ variant: string; mustContain: string[] }> = [
    { variant: "text", mustContain: ["Sample text content."] },
    { variant: "heading", mustContain: ["Sample heading"] },
    { variant: "badge", mustContain: ["Badge"] },
    { variant: "diff", mustContain: ["old line", "new line"] },
    { variant: "field_group", mustContain: ["Field group", "field control"] },
    { variant: "section", mustContain: ["Section", "section body"] },
    { variant: "array_table", mustContain: ["array_table", "Item one"] },
    { variant: "json_table", mustContain: ["json_table", "alpha"] },
    { variant: "list", mustContain: ["Item one", "Item three"] },
    { variant: "dialog", mustContain: ["Dialog title", "dialog body"] },
    { variant: "menu", mustContain: ["Open menu", "Item A"] },
    { variant: "tree", mustContain: ["root", "branch a", "leaf"] },
    { variant: "timeline", mustContain: ["09:01", "09:38"] },
    { variant: "markdown", mustContain: ["# Heading", "markdown"] },
    { variant: "rich_text", mustContain: ["rich text content"] },
    { variant: "markdown_editor", mustContain: ["Markdown editor"] },
    { variant: "ref_picker", mustContain: ["ref_picker", "Pick a reference"] },
    { variant: "detail", mustContain: ["Name", "Sample", "Status", "OK"] },
    { variant: "card", mustContain: ["Card title", "card body"] },
    { variant: "date_range", mustContain: ["date_range", "2026-01-01", "2026-01-31"] },
    { variant: "wizard", mustContain: ["① Step", "② Step", "③ Step"] },
    { variant: "drawer", mustContain: ["Drawer", "drawer body"] },
    { variant: "button", mustContain: ["Button"] },
    { variant: "text_field", mustContain: ["Text field", "Enter text"] },
    { variant: "number_field", mustContain: ["Number"] },
    { variant: "textarea", mustContain: ["Textarea", "Multi-line"] },
    { variant: "select_field", mustContain: ["Select", "Choose"] },
    { variant: "radio_group", mustContain: ["Radio group", "Option A", "Option B"] },
    { variant: "segmented", mustContain: ["Segmented", "One", "Two", "Three"] },
    { variant: "date_field", mustContain: ["Date", "YYYY-MM-DD"] },
    { variant: "checkbox", mustContain: ["Checkbox", "type=\"checkbox\""] },
    { variant: "action_widget", mustContain: ["Action", "Run"] },
  ];

  for (const { variant, mustContain } of NEW_PLACEHOLDERS) {
    it(`renders a faithful placeholder for ${variant}`, () => {
      const html = renderHarness(<PlaceholderRender node={{ type: variant }} />);
      expect(html).not.toContain("no placeholder yet");
      expect(html).toContain(`data-sdui-placeholder="${variant}"`);
      for (const fragment of mustContain) {
        expect(html).toContain(fragment);
      }
    });
  }
});
