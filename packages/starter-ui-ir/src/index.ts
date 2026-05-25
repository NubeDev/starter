// Hand-written TS mirror of `crates/starter-ui-ir/`. The Rust crate
// owns the JSON Schema artifact at
// `crates/starter-ui-ir/schema/starter-ui-ir.schema.json`; this file
// keeps the narrow set of shapes the renderer dispatches on in TS so
// consumers don't have to depend on the Rust workspace just to read
// IR types. See scope OQ-7 in `rubix/docs/scope/dashboards/`.

/** IR version the renderer speaks. Sent in every `ClientCapabilities`. */
export const IR_VERSION = 5 as const;

export type Kind =
  | "page"
  | "row"
  | "col"
  | "grid"
  | "stack"
  | "tabs"
  | "card"
  | "text"
  | "heading"
  | "badge"
  | "kpi"
  | "kpi_grid"
  | "button"
  | "link"
  | "table"
  | "form"
  | "field"
  | "select"
  | "toggle"
  | "slider"
  | "custom"
  | "chart"
  | "sparkline"
  | "divider"
  | "date_range"
  | "repeat";

export interface NodeStyle {
  className?: string;
  show_when?: ShowWhen;
}

export type ShowWhen =
  | { all: ShowWhen[] }
  | { any: ShowWhen[] }
  | { not: ShowWhen }
  | { eq: { path: string; value: unknown } }
  | { ne: { path: string; value: unknown } }
  | { truthy: { path: string } }
  | { falsy: { path: string } };

export interface UiComponent {
  type: string;
  id?: string;
  children?: UiComponent[];
  tabs?: { id?: string; label: string; children: UiComponent[] }[];
  style?: NodeStyle;
  [k: string]: unknown;
}

export interface UiComponentTree {
  ir_version: number;
  root: UiComponent;
}

export interface SubscriptionSubject {
  key: string;
  target_node_id: string;
  slot: string;
  field?: string;
  consumers: { component_id: string }[];
}

export interface SubscriptionPlan {
  subjects: SubscriptionSubject[];
}

export interface WritePlanEntry {
  component_id: string;
  handler: string;
  target_node_id?: string;
  slot?: string;
  field?: string;
}

export interface UiResolveResponseOk {
  render: UiComponentTree;
  subscriptions?: SubscriptionPlan;
  writes?: WritePlanEntry[];
}

export interface UiResolveResponseDryRun {
  errors: { location: string; message: string }[];
}

export type UiResolveResponse = UiResolveResponseOk | UiResolveResponseDryRun;

export interface Diagnostic {
  severity: "error" | "warning" | "info";
  code: string;
  message: string;
  field?: string;
}

export interface OptimisticHint {
  target_component_id: string;
  fields: Record<string, unknown>;
}

export type UiActionResponse =
  | { type: "noop" }
  | { type: "toast"; intent?: "info" | "success" | "warning" | "danger"; message: string }
  | { type: "redirect"; href: string }
  | { type: "patch"; target_id: string; fields: Record<string, unknown> }
  | { type: "full_render"; render: UiComponentTree }
  | { type: "dialog"; tree: UiComponent }
  | { type: "dismiss_dialog" }
  | { type: "diagnostics"; items: Diagnostic[] }
  | { type: "open_url"; href: string; target?: string };

export interface UiTableRow {
  id: string;
  kind?: string;
  path?: string;
  parent_id?: string;
  slots: Record<string, unknown>;
}

export interface ResolveRequest {
  page_ref: string;
  target_ref?: string;
  stack?: Record<string, string>;
  page_state?: Record<string, unknown>;
  capabilities?: ClientCapabilities;
}

export interface ClientCapabilities {
  ir_versions: number[];
  custom_renderers: string[];
}

export interface ActionRequest {
  handler: string;
  args?: Record<string, unknown>;
  context?: Record<string, unknown>;
}

export interface TableRequest {
  page_ref: string;
  component_id: string;
  cursor?: string;
  limit?: number;
}

export interface TableResponse {
  rows: UiTableRow[];
  next_cursor?: string;
}
