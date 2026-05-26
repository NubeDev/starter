// `PlaceholderRender` — palette / preview helper for non-runtime
// surfaces (e.g. the Puck builder's palette tiles, Storybook
// scenes). Given an IR node it produces a *synthetic* variant with
// representative data baked in and dispatches it through the same
// `Render` walker the live page uses. This keeps the palette tile
// visually identical to what the operator will see at runtime,
// without spinning up a transport.
//
// Per-variant fillers live in this file as a single map so adding a
// new placeholder is one entry, not a new file. Variants without a
// filler fall back to a minimal "variant tile" — visible breakage
// rather than silent re-use of partially-defined live data.
//
// Lives under `headless/` because it depends only on the renderer
// registry seam — mobile builds get the same behaviour as long as
// they register their own native renderers against the same
// registry.

import type { UiComponent } from "@nube/starter-ui-ir";
import { Render } from "./render.js";
import { lookupRenderer } from "./registry.js";

type PlaceholderFiller = (node: UiComponent) => UiComponent;

function sineSeries(n: number, amplitude = 50, base = 100): Array<[number, number]> {
  const out: Array<[number, number]> = [];
  for (let i = 0; i < n; i++) {
    const x = i;
    const y = base + amplitude * Math.sin((i / (n - 1)) * Math.PI * 2);
    out.push([x, Math.round(y * 10) / 10]);
  }
  return out;
}

const SAMPLE_TABLE_COLUMNS = [
  { key: "name", label: "Name" },
  { key: "value", label: "Value" },
  { key: "status", label: "Status" },
];

const SAMPLE_TABLE_ROWS = [
  { id: "1", name: "Item one", value: 12, status: "ok" },
  { id: "2", name: "Item two", value: 34, status: "warn" },
  { id: "3", name: "Item three", value: 56, status: "ok" },
];

const SAMPLE_KPI_GRID_ITEMS = [
  { id: "a", label: "Active", value: 12, unit_symbol: "" },
  { id: "b", label: "Pending", value: 4, unit_symbol: "" },
  { id: "c", label: "Errors", value: 0, unit_symbol: "" },
];

const FILLERS: Record<string, PlaceholderFiller> = {
  kpi: (node) => ({
    ...node,
    label: typeof node.label === "string" && node.label.length > 0 ? node.label : "Sample KPI",
    value: node.value ?? 123.4,
    unit_symbol:
      typeof node.unit_symbol === "string" ? node.unit_symbol : "kWh",
    format: typeof node.format === "string" ? node.format : "number",
    trend: typeof node.trend === "string" ? node.trend : "+12%",
  }),
  chart: (node) => ({
    ...node,
    title: typeof node.title === "string" && node.title.length > 0 ? node.title : "Sample chart",
    series: Array.isArray(node.series) && node.series.length > 0
      ? node.series
      : [{ points: sineSeries(6) }],
  }),
  sparkline: (node) => ({
    ...node,
    series: Array.isArray(node.series) && node.series.length > 0
      ? node.series
      : [{ points: sineSeries(6) }],
  }),
  table: (node) => ({
    ...node,
    columns: Array.isArray(node.columns) && node.columns.length > 0
      ? node.columns
      : SAMPLE_TABLE_COLUMNS,
    rows: Array.isArray(node.rows) && node.rows.length > 0
      ? node.rows
      : SAMPLE_TABLE_ROWS,
  }),
  kpi_grid: (node) => ({
    ...node,
    columns: typeof node.columns === "number" && node.columns > 0 ? node.columns : 3,
    items: Array.isArray(node.items) && node.items.length > 0
      ? node.items
      : SAMPLE_KPI_GRID_ITEMS,
  }),
  repeat: (node) => {
    // The live `RenderRepeat` requires `node.template`. For the
    // palette tile we synthesise a tiny KPI template and three
    // rows so the operator sees "the shape" without authoring.
    const template: UiComponent =
      (node.template as UiComponent | undefined) ?? {
        type: "kpi",
        label: "Sample",
        value: 1,
      };
    const items = Array.isArray(node.items) && node.items.length > 0
      ? node.items
      : [{}, {}, {}];
    return { ...node, template, items };
  },
  form: (node) => ({
    ...node,
    // The live `RenderForm` reads `submit.handler`; a placeholder
    // shouldn't dispatch anything, so we drop `submit` and let the
    // form render as a child-only frame.
    submit: undefined,
    children: Array.isArray(node.children) && node.children.length > 0
      ? node.children
      : [
          { type: "text_field", id: "name", label: "Name" } as UiComponent,
          { type: "text_field", id: "email", label: "Email" } as UiComponent,
        ],
  }),
};

/**
 * Dispatch `node` through the live renderer registry with
 * synthetic data filled in for variants that ship a filler. For
 * variants without a filler, falls through to `Render` unchanged
 * (which itself dispatches if a renderer is registered or shows
 * the dangling-variant placeholder otherwise).
 *
 * Used by surfaces that need to preview a widget without a
 * transport (palette tiles, design-system scenes). Not used by the
 * live page — `SduiPage` uses `Render` directly so server-resolved
 * data is authoritative.
 */
export function PlaceholderRender({ node }: { node: UiComponent }) {
  const filler = FILLERS[node.type];
  const filled = filler ? filler(node) : node;
  // If neither a filler nor a renderer is registered, show a
  // dashed tile with the variant name so the palette surface
  // doesn't silently render nothing.
  if (!filler && !lookupRenderer(node.type)) {
    return (
      <div
        data-sdui-placeholder-missing={node.type}
        style={{
          padding: 12,
          border: "1px dashed #888",
          borderRadius: 4,
          color: "#555",
          fontSize: 12,
          fontFamily: "ui-sans-serif, system-ui",
        }}
      >
        <div style={{ fontWeight: 600 }}>{node.type}</div>
        <div style={{ opacity: 0.7 }}>no placeholder yet</div>
      </div>
    );
  }
  return <Render node={filled} />;
}
