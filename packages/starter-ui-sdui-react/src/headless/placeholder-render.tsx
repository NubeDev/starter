// `PlaceholderRender` — palette / preview helper for non-runtime
// surfaces (e.g. the Puck builder's palette tiles, Storybook
// scenes). Given an IR node it produces a *synthetic* variant with
// representative data baked in and dispatches it through the same
// `Render` walker the live page uses. This keeps the palette tile
// visually identical to what the operator will see at runtime,
// without spinning up a transport.
//
// Per-variant fillers live in this file as two maps:
//   * `FILLERS`  — variants that have a live renderer; the filler
//                  injects sample data and we dispatch through
//                  `Render` so the placeholder tile is pixel-equal
//                  to runtime.
//   * `VISUALS`  — variants that have NO live web renderer yet (or
//                  whose live renderer needs a transport). These
//                  return JSX directly that mirrors the live
//                  renderer's visual idiom (axes for chart_kind
//                  variants, header+rows for table variants, etc).
//
// Variants without an entry in either map fall back to a minimal
// "variant tile" — visible breakage rather than silent re-use of
// partially-defined live data.
//
// Lives under `headless/` because it depends only on the renderer
// registry seam — mobile builds get the same behaviour as long as
// they register their own native renderers against the same
// registry.

import type { CSSProperties, ReactNode } from "react";
import type { UiComponent } from "@nube/starter-ui-ir";
import { Render } from "./render.js";
import { lookupRenderer } from "./registry.js";

type PlaceholderFiller = (node: UiComponent) => UiComponent;
type PlaceholderVisual = (node: UiComponent) => ReactNode;

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

// --- visuals --------------------------------------------------------------

const BOX: CSSProperties = {
  padding: 8,
  border: "1px solid #d0d0d0",
  borderRadius: 4,
  fontFamily: "ui-sans-serif, system-ui",
  fontSize: 12,
  background: "#fff",
};

const LABEL: CSSProperties = {
  fontSize: 11,
  color: "#666",
  marginBottom: 4,
};

const INPUT: CSSProperties = {
  padding: "4px 6px",
  border: "1px solid #ccc",
  borderRadius: 3,
  background: "#fafafa",
  color: "#999",
  fontSize: 12,
};

function str(v: unknown, fallback: string): string {
  return typeof v === "string" && v.length > 0 ? v : fallback;
}

const VISUALS: Record<string, PlaceholderVisual> = {
  text: (node) => (
    <p data-sdui-placeholder="text" style={{ margin: 0, fontSize: 13, color: "#222" }}>
      {str(node.content, "Sample text content.")}
    </p>
  ),
  heading: (node) => {
    const level = typeof node.level === "number" ? Math.min(6, Math.max(1, node.level)) : 2;
    const size = [22, 20, 18, 16, 14, 13][level - 1];
    return (
      <div data-sdui-placeholder="heading">
        <div style={{ fontWeight: 600, fontSize: size, color: "#111" }}>
          {str(node.content, "Sample heading")}
        </div>
        {node.subtitle ? (
          <div style={{ fontSize: 12, color: "#666" }}>{String(node.subtitle)}</div>
        ) : null}
      </div>
    );
  },
  badge: (node) => (
    <span
      data-sdui-placeholder="badge"
      style={{
        display: "inline-block",
        padding: "2px 8px",
        borderRadius: 999,
        background: "#eef",
        color: "#225",
        fontSize: 11,
        fontWeight: 500,
      }}
    >
      {str(node.label, "Badge")}
    </span>
  ),
  diff: (node) => (
    <div data-sdui-placeholder="diff" style={{ ...BOX, fontFamily: "ui-monospace, monospace" }}>
      <div style={{ background: "#ffecec", padding: 2 }}>- {str(node.old_text, "old line")}</div>
      <div style={{ background: "#e6ffed", padding: 2 }}>+ {str(node.new_text, "new line")}</div>
    </div>
  ),
  field_group: (node) => (
    <div data-sdui-placeholder="field_group" style={BOX}>
      <div style={LABEL}>{str(node.label, "Field group")}</div>
      <div style={INPUT}>field control</div>
      {node.helper ? <div style={{ ...LABEL, marginTop: 4 }}>{String(node.helper)}</div> : null}
    </div>
  ),
  section: (node) => (
    <section data-sdui-placeholder="section" style={{ ...BOX, padding: 12 }}>
      <div style={{ fontWeight: 600, marginBottom: 4 }}>{str(node.title, "Section")}</div>
      {node.subtitle ? (
        <div style={{ ...LABEL, marginBottom: 8 }}>{String(node.subtitle)}</div>
      ) : null}
      <div style={{ color: "#999", fontSize: 12 }}>(section body)</div>
    </section>
  ),
  array_table: () => (
    <div data-sdui-placeholder="array_table" style={BOX}>
      <div style={LABEL}>array_table</div>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr style={{ background: "#f4f4f4" }}>
            <th style={{ textAlign: "left", padding: 4 }}>Name</th>
            <th style={{ textAlign: "left", padding: 4 }}>Value</th>
          </tr>
        </thead>
        <tbody>
          {SAMPLE_TABLE_ROWS.slice(0, 2).map((r) => (
            <tr key={r.id}>
              <td style={{ padding: 4 }}>{r.name}</td>
              <td style={{ padding: 4 }}>{r.value}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  ),
  json_table: () => (
    <div data-sdui-placeholder="json_table" style={BOX}>
      <div style={LABEL}>json_table</div>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr style={{ background: "#f4f4f4" }}>
            <th style={{ textAlign: "left", padding: 4 }}>key</th>
            <th style={{ textAlign: "left", padding: 4 }}>value</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td style={{ padding: 4 }}>alpha</td>
            <td style={{ padding: 4 }}>1</td>
          </tr>
          <tr>
            <td style={{ padding: 4 }}>beta</td>
            <td style={{ padding: 4 }}>2</td>
          </tr>
        </tbody>
      </table>
    </div>
  ),
  list: () => (
    <ul data-sdui-placeholder="list" style={{ ...BOX, listStyle: "disc", paddingLeft: 24 }}>
      <li>Item one</li>
      <li>Item two</li>
      <li>Item three</li>
    </ul>
  ),
  dialog: (node) => (
    <div data-sdui-placeholder="dialog" style={{ ...BOX, boxShadow: "0 2px 8px rgba(0,0,0,0.1)" }}>
      <div style={{ fontWeight: 600 }}>{str(node.title, "Dialog title")}</div>
      {node.description ? (
        <div style={{ ...LABEL, marginTop: 4 }}>{String(node.description)}</div>
      ) : null}
      <div style={{ marginTop: 8, color: "#999" }}>(dialog body)</div>
    </div>
  ),
  menu: () => (
    <div data-sdui-placeholder="menu" style={BOX}>
      <div style={LABEL}>menu</div>
      <div style={{ padding: 4 }}>▾ Open menu</div>
      <div style={{ padding: 4, color: "#666" }}>· Item A</div>
      <div style={{ padding: 4, color: "#666" }}>· Item B</div>
    </div>
  ),
  tree: () => (
    <div data-sdui-placeholder="tree" style={{ ...BOX, fontFamily: "ui-monospace, monospace" }}>
      <div>▾ root</div>
      <div style={{ paddingLeft: 12 }}>▸ branch a</div>
      <div style={{ paddingLeft: 12 }}>▾ branch b</div>
      <div style={{ paddingLeft: 24 }}>· leaf</div>
    </div>
  ),
  timeline: () => (
    <div data-sdui-placeholder="timeline" style={BOX}>
      <div style={LABEL}>timeline</div>
      {["09:01 event", "09:14 event", "09:38 event"].map((e) => (
        <div key={e} style={{ display: "flex", gap: 8, padding: "2px 0" }}>
          <span style={{ color: "#999" }}>●</span>
          <span>{e}</span>
        </div>
      ))}
    </div>
  ),
  markdown: (node) => (
    <div data-sdui-placeholder="markdown" style={BOX}>
      <div style={{ fontWeight: 600, fontSize: 14 }}># Heading</div>
      <div>{str(node.content, "Some **markdown** body text.")}</div>
    </div>
  ),
  rich_text: (node) => (
    <div data-sdui-placeholder="rich_text" style={BOX}>
      <em>{str(node.value, "rich text content")}</em>
    </div>
  ),
  markdown_editor: (node) => (
    <div data-sdui-placeholder="markdown_editor" style={BOX}>
      <div style={LABEL}>{str(node.label, "Markdown editor")}</div>
      <textarea
        readOnly
        style={{ ...INPUT, width: "100%", minHeight: 60 }}
        value={str(node.value, "# Markdown\nedit me")}
      />
    </div>
  ),
  ref_picker: (node) => (
    <div data-sdui-placeholder="ref_picker" style={BOX}>
      <div style={LABEL}>ref_picker</div>
      <div style={INPUT}>{str(node.placeholder, "Pick a reference…")} ▾</div>
    </div>
  ),
  detail: () => (
    <dl data-sdui-placeholder="detail" style={{ ...BOX, display: "grid", gridTemplateColumns: "auto 1fr", gap: 4 }}>
      <dt style={{ color: "#666" }}>Name</dt>
      <dd style={{ margin: 0 }}>Sample</dd>
      <dt style={{ color: "#666" }}>Status</dt>
      <dd style={{ margin: 0 }}>OK</dd>
    </dl>
  ),
  card: (node) => (
    <div data-sdui-placeholder="card" style={{ ...BOX, padding: 12 }}>
      <div style={{ fontWeight: 600 }}>{str(node.title, "Card title")}</div>
      {node.subtitle ? (
        <div style={LABEL}>{String(node.subtitle)}</div>
      ) : null}
      <div style={{ marginTop: 8, color: "#666" }}>(card body)</div>
    </div>
  ),
  date_range: () => (
    <div data-sdui-placeholder="date_range" style={BOX}>
      <div style={LABEL}>date_range</div>
      <div style={{ display: "flex", gap: 4 }}>
        <span style={INPUT}>2026-01-01</span>
        <span style={{ alignSelf: "center" }}>→</span>
        <span style={INPUT}>2026-01-31</span>
      </div>
    </div>
  ),
  wizard: () => (
    <div data-sdui-placeholder="wizard" style={BOX}>
      <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
        <span style={{ fontWeight: 600 }}>① Step</span>
        <span style={{ color: "#999" }}>② Step</span>
        <span style={{ color: "#999" }}>③ Step</span>
      </div>
      <div style={{ color: "#999" }}>(step body)</div>
    </div>
  ),
  drawer: (node) => (
    <div data-sdui-placeholder="drawer" style={{ ...BOX, borderLeft: "3px solid #888" }}>
      <div style={{ fontWeight: 600 }}>{str(node.title, "Drawer")}</div>
      <div style={{ marginTop: 4, color: "#999" }}>(drawer body)</div>
    </div>
  ),
  button: (node) => (
    <button
      type="button"
      data-sdui-placeholder="button"
      style={{
        padding: "4px 10px",
        background: "#0a66c2",
        color: "#fff",
        border: 0,
        borderRadius: 3,
        fontSize: 12,
      }}
    >
      {str(node.label, "Button")}
    </button>
  ),
  text_field: (node) => (
    <div data-sdui-placeholder="text_field">
      <div style={LABEL}>{str(node.label, "Text field")}</div>
      <div style={INPUT}>{str(node.placeholder, "Enter text…")}</div>
    </div>
  ),
  number_field: (node) => (
    <div data-sdui-placeholder="number_field">
      <div style={LABEL}>{str(node.label, "Number")}</div>
      <div style={INPUT}>{str(node.placeholder, "0")}</div>
    </div>
  ),
  textarea: (node) => (
    <div data-sdui-placeholder="textarea">
      <div style={LABEL}>{str(node.label, "Textarea")}</div>
      <div style={{ ...INPUT, minHeight: 48 }}>{str(node.placeholder, "Multi-line text…")}</div>
    </div>
  ),
  select_field: (node) => (
    <div data-sdui-placeholder="select_field">
      <div style={LABEL}>{str(node.label, "Select")}</div>
      <div style={INPUT}>{str(node.placeholder, "Choose…")} ▾</div>
    </div>
  ),
  radio_group: (node) => (
    <div data-sdui-placeholder="radio_group">
      <div style={LABEL}>{str(node.label, "Radio group")}</div>
      <label style={{ display: "block" }}>○ Option A</label>
      <label style={{ display: "block" }}>● Option B</label>
      <label style={{ display: "block" }}>○ Option C</label>
    </div>
  ),
  segmented: (node) => (
    <div data-sdui-placeholder="segmented">
      <div style={LABEL}>{str(node.label, "Segmented")}</div>
      <div style={{ display: "inline-flex", border: "1px solid #ccc", borderRadius: 3, overflow: "hidden" }}>
        <span style={{ padding: "4px 8px", background: "#eee" }}>One</span>
        <span style={{ padding: "4px 8px", background: "#0a66c2", color: "#fff" }}>Two</span>
        <span style={{ padding: "4px 8px", background: "#eee" }}>Three</span>
      </div>
    </div>
  ),
  date_field: (node) => (
    <div data-sdui-placeholder="date_field">
      <div style={LABEL}>{str(node.label, "Date")}</div>
      <div style={INPUT}>📅 {str(node.placeholder, "YYYY-MM-DD")}</div>
    </div>
  ),
  checkbox: (node) => (
    <label data-sdui-placeholder="checkbox" style={{ display: "inline-flex", gap: 6, alignItems: "center" }}>
      <input type="checkbox" readOnly />
      <span>{str(node.label, "Checkbox")}</span>
    </label>
  ),
  action_widget: (node) => (
    <div data-sdui-placeholder="action_widget" style={{ ...BOX, padding: 12 }}>
      <div style={{ fontWeight: 600 }}>{str(node.title, "Action")}</div>
      {node.description ? (
        <div style={LABEL}>{String(node.description)}</div>
      ) : null}
      <button
        type="button"
        style={{
          marginTop: 6,
          padding: "4px 10px",
          background: "#0a66c2",
          color: "#fff",
          border: 0,
          borderRadius: 3,
          fontSize: 12,
        }}
      >
        Run
      </button>
    </div>
  ),
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
  // Visuals take precedence — these are for variants that have no
  // live renderer (or whose live renderer can't run without a
  // transport) and we draw the placeholder JSX directly.
  const visual = VISUALS[node.type];
  if (visual) {
    return <>{visual(node)}</>;
  }
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
