// Data-source selectors (§B3) — typed leaves that should render as a
// catalogue-backed picker (analytics templates, tool refs, tenants,
// unit symbols, page-state keys) instead of the default text field.
//
// The IR schema does not carry this signal — it is curated next to
// the generator, in the same spirit as `bindable.ts` and `slots.ts`.
// Each entry is a tuple `(variant, propertyPath, kind)` where `kind`
// selects one of the catalogue verbs the host (rubix-frontend / the
// harness) provides via the `Catalogue` seam.
//
// New entries are cheap — list them here and the generator will route
// them through `<DataSourceField>` automatically. Mismatched paths
// silently fall through to the default field; if a picker fails to
// appear, the tuple here is the first place to check.

/** The set of catalogue lookups the host can satisfy. */
export type CatalogueKind =
  | "analytics_template" // analytics template stems — populated from rubix.analytics.template.list (or equivalent verb)
  | "tool" // tool refs — populated from /api/v1/tools
  | "tenant" // tenant ids — populated from rubix.tenant.list
  | "unit_symbol" // unit symbols (display-only; kWh, °C, …)
  | "page_state_key"; // $page.<key> dotted paths used in BindingSpec strings

export type DataSourceTuple = {
  /** Snake-case IR `type` discriminator. */
  variant: string;
  /** Dotted path inside the variant's props. */
  propertyPath: string;
  /** Catalogue kind to render as. */
  kind: CatalogueKind;
};

/**
 * v1 list. The IR uses string-typed leaves (the IR doesn't carry
 * dedicated `AnalyticsTemplateRef`/`ToolRef` types), so we list the
 * `(variant, propertyPath)` pairs that semantically *should* be
 * pickers. Anything not listed renders with the schema-derived
 * default — usually `text`, which is the prior PR1 behaviour.
 */
export const DATA_SOURCES: readonly DataSourceTuple[] = [
  // KPI's `unit_symbol` (display-only string, common-unit suggestions).
  { variant: "kpi", propertyPath: "unit_symbol", kind: "unit_symbol" },
  // Same field on the sparkline tile.
  { variant: "sparkline", propertyPath: "unit_symbol", kind: "unit_symbol" },

  // KPI's `source` resolves through ChartSource → currently falls
  // through to a text fallback because flatten() hits a `oneOf`.
  // Route through the analytics-template picker so the operator can
  // pick a template stem directly. (Selecting a non-analytics_template
  // source shape is deferred to the union picker in B3 PR2.)
  { variant: "kpi", propertyPath: "source", kind: "analytics_template" },

  // Action target shape — handler name resolves to a registered tool.
  { variant: "action_widget", propertyPath: "action_ref", kind: "tool" },

  // Show-when binding strings — page-state key dropdown.
  // (NodeStyle.show_when itself is skipped via SKIPPED_PROPERTIES,
  // but `drawer.open` is a bindable string per BINDABLE.)
  { variant: "drawer", propertyPath: "open", kind: "page_state_key" },
] as const;

/**
 * True when (variant, propertyPath) is registered as a catalogue-
 * backed field. The generator consults this BEFORE its default
 * type-driven dispatch.
 */
export function dataSourceKindOf(
  variant: string,
  propertyPath: string,
): CatalogueKind | undefined {
  for (const t of DATA_SOURCES) {
    if (t.variant === variant && t.propertyPath === propertyPath) return t.kind;
  }
  return undefined;
}
