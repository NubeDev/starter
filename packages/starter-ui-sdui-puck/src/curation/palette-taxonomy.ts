// Palette taxonomy — which palette bucket each IR variant sits in.
// The Puck palette is generated from the schema (scope §B2: "not
// hand-curated") but the *grouping* IS hand-curated here. Variants
// absent from the map render as "uncategorised" in the palette —
// scope §B2 calls this "visible breakage, not silent drift."
//
// Bucket names come from the user-facing palette UI per scope §B1:
//   - "layout"      — containers that hold other widgets
//   - "display"     — read-only widgets that present data
//   - "interactive" — write-path / input widgets
//   - "custom"      — extension-registered (`Component::Custom`)
//
// Resolver-only variants (`forbidden`, `dangling`, `unknown`) are
// not classified here at all — they are filtered out *before* the
// taxonomy lookup. See OVERRIDES guard in `overrides.ts`.

export type PaletteCategory = "layout" | "display" | "interactive" | "custom";

export const PALETTE_TAXONOMY: Readonly<Record<string, PaletteCategory>> = {
  // ---- layout ---------------------------------------------------
  page: "layout",
  row: "layout",
  col: "layout",
  grid: "layout",
  tabs: "layout",
  section: "layout",
  divider: "layout",
  repeat: "layout",
  drawer: "layout",
  dialog: "layout",
  wizard: "layout",
  card: "layout",
  field_group: "layout",
  hero: "layout",
  spacer: "layout",
  // ---- display --------------------------------------------------
  text: "display",
  heading: "display",
  badge: "display",
  image: "display",
  markdown: "display",
  rich_text: "display",
  kpi: "display",
  kpi_grid: "display",
  chart: "display",
  sparkline: "display",
  table: "display",
  array_table: "display",
  json_table: "display",
  list: "display",
  tree: "display",
  timeline: "display",
  detail: "display",
  diff: "display",
  menu: "display",
  action_widget: "display",
  // ---- interactive (write-path) --------------------------------
  form: "interactive",
  text_field: "interactive",
  number_field: "interactive",
  textarea: "interactive",
  toggle: "interactive",
  slider: "interactive",
  checkbox: "interactive",
  select: "interactive",
  select_field: "interactive",
  radio_group: "interactive",
  segmented: "interactive",
  date_field: "interactive",
  date_range: "interactive",
  ref_picker: "interactive",
  markdown_editor: "interactive",
  button: "interactive",
  // ---- custom (extension-registered) ----------------------------
  custom: "custom",
};

/**
 * Look up the bucket for an IR variant. Returns `undefined` for
 * uncategorised variants — the palette UI should surface them under
 * an "Uncategorised" group so the missing classification is visible.
 */
export function categoryFor(variant: string): PaletteCategory | undefined {
  return PALETTE_TAXONOMY[variant];
}
