// Kind-specific authoring config carried in a variable's opaque
// `options_config` jsonb (WS-02). The wire treats this as `unknown` because
// its shape varies by `kind` and the UI owns each shape; this module is the
// single place that reads/writes those shapes, with defensive parsing so a
// malformed blob degrades to an empty config rather than throwing.

import type { ContextSource, VariableKind } from "@/data/types";

/** `constant` — one fixed value (usually hidden). */
export interface ConstantConfig {
  value: string;
}

/** `custom` — a static comma-separated option list, authored as text.
 *  Each entry may be `value` or `text : value` (Grafana syntax). */
export interface CustomConfig {
  /** Raw authoring text, e.g. `prod, staging, Dev : dev`. */
  optionsText: string;
}

/** `query` — options come from running SQL against a datasource. The SQL
 *  may reference other variables (`$parent`), which makes it cascade. */
export interface QueryConfig {
  datasourceId: string;
  sql: string;
  /** Optional result column for the displayed label; defaults to the
   *  same column as the value (the first column). */
  textColumn?: string;
  /** Result column for the bound value; defaults to the first column. */
  valueColumn?: string;
}

/** `datasource` — options are the tenant's datasources, optionally
 *  filtered to one kind (so `$ds` can drive panels). */
export interface DatasourceConfig {
  /** Restrict to datasources of this kind (e.g. `postgres`); empty = all.
   *  Named `kindFilter` (not `kind`) to avoid clashing with the
   *  discriminant on the tagged union. */
  kindFilter?: string;
}

/** `interval` — a list of durations (drives `$__interval` overrides). */
export interface IntervalConfig {
  /** e.g. `["1m", "5m", "1h"]`. */
  steps: string[];
}

/** `textbox` — free text; the config holds only the default value. */
export interface TextboxConfig {
  default: string;
}

/** `context` (WS-13) — read a value from the page's context. `source` names
 *  one of the four `PageContext` slots; `key` selects within it:
 *  - `nav` + `slug|name|path[n]` → the nav node's slug/name/ancestor
 *  - `url` + `building` → the bare `?building=…` param
 *  - `tag` + `building` → the dashboard's `building` tag
 *  - `values` + `building` → the nav node's `context.values[building]`
 *  Resolution is synchronous against the assembled `PageContext`. */
export interface ContextConfig {
  source: ContextSource;
  key: string;
}

export type KindConfig =
  | ({ kind: "constant" } & ConstantConfig)
  | ({ kind: "custom" } & CustomConfig)
  | ({ kind: "query" } & QueryConfig)
  | ({ kind: "datasource" } & DatasourceConfig)
  | ({ kind: "interval" } & IntervalConfig)
  | ({ kind: "textbox" } & TextboxConfig)
  | ({ kind: "context" } & ContextConfig);

function asRecord(raw: unknown): Record<string, unknown> {
  return raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
}

function str(o: Record<string, unknown>, key: string, fallback = ""): string {
  const v = o[key];
  return typeof v === "string" ? v : fallback;
}

/** Parse the opaque `options_config` into a typed, kind-tagged config,
 *  defaulting every field so a missing/garbled blob yields an empty config
 *  of the right shape rather than throwing. */
export function parseKindConfig(kind: VariableKind, raw: unknown): KindConfig {
  const o = asRecord(raw);
  switch (kind) {
    case "constant":
      return { kind, value: str(o, "value") };
    case "custom":
      return { kind, optionsText: str(o, "optionsText") };
    case "query":
      return {
        kind,
        datasourceId: str(o, "datasourceId"),
        sql: str(o, "sql"),
        textColumn: typeof o.textColumn === "string" ? o.textColumn : undefined,
        valueColumn:
          typeof o.valueColumn === "string" ? o.valueColumn : undefined,
      };
    case "datasource":
      return { kind, kindFilter: str(o, "kindFilter") || undefined };
    case "interval": {
      const steps = Array.isArray(o.steps)
        ? o.steps.filter((s): s is string => typeof s === "string")
        : [];
      return { kind, steps };
    }
    case "textbox":
      return { kind, default: str(o, "default") };
    case "context":
      return { kind, source: contextSource(o.source), key: str(o, "key") };
    default:
      return { kind: "custom", optionsText: "" };
  }
}

/** Coerce a stored `source` to a valid [`ContextSource`], defaulting to `url`
 *  (the most external, deep-link-friendly source) for a missing/garbled value
 *  so a malformed blob never throws. */
function contextSource(raw: unknown): ContextSource {
  return raw === "nav" || raw === "tag" || raw === "values" ? raw : "url";
}

/** Serialise a typed config back to the opaque jsonb stored in
 *  `options_config`. Strips the discriminant `kind` (it lives in the
 *  variable's own `kind` column). */
export function toOptionsConfig(config: KindConfig): Record<string, unknown> {
  const { kind: _kind, ...rest } = config;
  return rest;
}
