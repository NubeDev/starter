/**
 * Wire-shape types for the SDUI renderer.
 *
 * These mirror the JSON Schema emitted by `starter-ui-ir` at build
 * time. They are intentionally narrow: the renderer only ever reads
 * the fields it dispatches on (`type`, `id`, `children`, `style`,
 * `show_when`) and treats the rest opaquely. When the IR adds a
 * variant the renderer doesn't yet know, dispatch falls through to a
 * "Unknown component" placeholder rather than throwing.
 *
 * Diagnostics is the wider `{ severity, code, message, field? }`
 * shape — divergence **D1** from Rubix. Starter does not accept
 * `form_errors` at the wire.
 */

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
  | "custom";

export interface UiComponent {
  type: string;
  id?: string;
  children?: UiComponent[];
  tabs?: { id?: string; label: string; children: UiComponent[] }[];
  style?: NodeStyle;
  [k: string]: unknown;
}

export interface NodeStyle {
  className?: string;
  show_when?: ShowWhen;
}

/**
 * Page-state predicate. Resolved on the client (the renderer reads
 * `pageState` from context). Server-resolved bindings are flattened
 * into the IR before it reaches the renderer (R4).
 */
export type ShowWhen =
  | { all: ShowWhen[] }
  | { any: ShowWhen[] }
  | { not: ShowWhen }
  | { eq: { path: string; value: unknown } }
  | { ne: { path: string; value: unknown } }
  | { truthy: { path: string } }
  | { falsy: { path: string } };

export interface UiComponentTree {
  ir_version: number;
  root: UiComponent;
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

export interface SubscriptionPlan {
  subjects: SubscriptionSubject[];
}

export interface SubscriptionSubject {
  /** Unique key (e.g. `target_node:slot[.field]`). */
  key: string;
  target_node_id: string;
  slot: string;
  field?: string;
  /** Component IDs that consume this subject. */
  consumers: { component_id: string }[];
}

export interface UiTableRow {
  id: string;
  kind?: string;
  path?: string;
  parent_id?: string;
  slots: Record<string, unknown>;
}

/** Diagnostic item — divergence D1 (replaces Rubix `form_errors`). */
export interface Diagnostic {
  severity: "error" | "warning" | "info";
  code: string;
  message: string;
  field?: string;
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
