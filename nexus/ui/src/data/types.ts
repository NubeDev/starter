// The dashboard/widget data model — the stable contract that survives
// every layer swap (F7). It is intentionally free of React, the chart
// library, and the transport: a panel is described by *what data it
// wants* (a datasource + query + field mapping) and *how to draw it* (a
// widget type + display options), never by a provider-specific handle.
//
// This shape is also what an OpenUI "Ask Nexus" generator emits, so it
// stays schema-describable and side-effect-free.

export type WidgetType =
  | "line"
  | "area"
  | "gauge"
  | "stat"
  | "status"
  | "table"
  | "pie"
  | "bar"
  | "scatter"
  | "heatmap";

export type Trend = "up" | "down" | "flat";

export interface WidgetLayout {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Where a panel's rows come from: a saved datasource plus the SQL that
 *  runs against it. Executed by `nexus-api`'s `POST /query` — the UI
 *  never holds credentials or talks to a database directly. */
export interface PanelQuery {
  datasourceId: string;
  sql: string;
  /** Bound query parameters, positional (`$1`, `$2`, …). */
  params?: ReadonlyArray<string | number | boolean | null>;
  /** Kind-mode (WS-10): a declarative query-kind invoked by reverse-DNS
   *  name (`nexus.core.meters_list`) instead of raw `sql`. When set, the
   *  backend resolves the kind from its registry, validates `kindParams`
   *  against the kind's JSON Schema, and binds the kind's SQL — `sql` is
   *  ignored. The host-bound tenant predicate is injected server-side and
   *  is never supplied here. */
  kind?: string;
  /** Named params for a kind-mode query, keyed by the schema's property
   *  names. Scalars only (the binder binds each as a single arg). Ignored
   *  when `kind` is unset. */
  kindParams?: Readonly<Record<string, string | number | boolean>>;
  /** Optional post-query insight (RW-06): a saved insight id whose Rhai
   *  transform is applied to this query's result before it reaches the widget.
   *  The panel still owns the SQL + datasource; the insight is the reusable
   *  lens on top. Unset = the raw query result is rendered. Sent to the query
   *  endpoint as `{ insight: { insight_id } }`. */
  insightId?: string;
  /** Params bound as the insight script's `params` object. Ignored when
   *  `insightId` is unset. */
  insightParams?: unknown;
}

/** One drawn series, mapped from a column in the query result. */
export interface SeriesField {
  /** Result column holding the numeric value. */
  value: string;
  label?: string;
  unit?: string;
  /** hsl string, e.g. "152 76% 44%"; defaults to the chart palette. */
  color?: string;
  /** How a *table* should render this column's cells. `"time"` routes
   *  the raw value through the region/preference date formatter
   *  (`useDateTime`); `"number"` / `"text"` (default) render as-is.
   *  Charts ignore this — series values are always numeric. */
  kind?: "time" | "number" | "text";
}

/** Maps query-result columns onto chart roles. The `x` column is the
 *  category/time axis for line/area; omitted for single-value panels
 *  (stat/gauge) that read only the first series. */
export interface FieldMapping {
  x?: string;
  series: ReadonlyArray<SeriesField>;
  /** How the `x` column is interpreted for display. `"time"` formats
   *  axis labels + tooltip headers through the active region/preference
   *  date formatter; `"category"` (default) prints the raw value. The
   *  axis values themselves stay raw either way — only the rendered
   *  label is formatted, so ordering and spacing are unaffected. */
  xKind?: "time" | "category";
}

/** Gauge/stat threshold bounds. `crit < warn` encodes a descending
 *  metric (battery SoC), `crit > warn` an ascending one (load). */
export interface Thresholds {
  warn?: number;
  crit?: number;
}

/** One step in a multi-step threshold ramp: at or above `value` (for an
 *  ascending metric) the reading takes `color`. The lowest step's
 *  `value` is the base colour (conventionally `-Infinity`, serialised as
 *  `null`). Steps are kept sorted ascending by `value` by the editor;
 *  consumers should not assume order and sort defensively. */
export interface ThresholdStep {
  /** Lower bound of this step. `null` means "the base step" (no lower
   *  bound) so it survives JSON round-trips (`-Infinity` does not). */
  value: number | null;
  /** hsl string, e.g. "152 76% 44%". */
  color: string;
}

/** Maps a raw value (or a numeric range, or a regex over text) onto a
 *  display text and/or colour — Grafana's "value mappings". The first
 *  matching mapping wins. */
export interface ValueMapping {
  /** Exact-value match (compared as string after coercion). */
  type: "value" | "range" | "regex";
  /** For `value`: the literal. For `regex`: the pattern. */
  match?: string;
  /** For `range`: inclusive bounds (either may be omitted for open-ended). */
  from?: number;
  to?: number;
  /** Replacement display text. */
  text?: string;
  /** hsl string applied when this mapping matches. */
  color?: string;
}

/** Per-field display configuration — units, precision, bounds, the
 *  threshold ramp, and value mappings. This is the "defaults" half of
 *  {@link FieldConfig}; an {@link FieldOverride} carries the same shape
 *  to selectively replace it per series. All fields optional so an
 *  untouched panel serialises to `{}` and reads back identically. */
export interface FieldDisplay {
  /** Unit id from the unit registry (`features/widgets/units.ts`), e.g.
   *  `"celsius"`, `"percent"`, `"watt"`. Undefined → unitless. */
  unit?: string;
  /** Fixed decimal places for the formatted value. Undefined → auto. */
  decimals?: number;
  min?: number;
  max?: number;
  /** What to show when there is no value (defaults to an em dash). */
  noValue?: string;
  /** Multi-step colour ramp; supersedes the legacy {@link Thresholds}
   *  when present. Empty/undefined → no threshold colouring. */
  thresholds?: ReadonlyArray<ThresholdStep>;
  mappings?: ReadonlyArray<ValueMapping>;
}

/** A per-series/per-column override: when a series matches `matcher`,
 *  its display config is the field defaults with these properties laid
 *  on top. */
export interface FieldOverride {
  matcher: FieldMatcher;
  /** Properties to override; also allows hiding a series and renaming
   *  its display label, beyond the {@link FieldDisplay} props. */
  display: FieldDisplay & { displayName?: string; hidden?: boolean; color?: string };
}

/** How an override selects the series it applies to. `byName` matches a
 *  series' `value` column (or its label) exactly; `byRegex` tests the
 *  same against a regular expression. */
export interface FieldMatcher {
  type: "byName" | "byRegex";
  /** The column name (or label) for `byName`, the pattern for `byRegex`. */
  value: string;
}

/** Grafana-style field config: a default display applied to every series
 *  plus targeted overrides. Lives on {@link WidgetConfig} and is read by
 *  the option-builders (after resolution) and the table/stat renderers. */
export interface FieldConfig {
  defaults?: FieldDisplay;
  overrides?: ReadonlyArray<FieldOverride>;
}

/** Legend display options for multi-series charts. */
export interface LegendOptions {
  show?: boolean;
  placement?: "top" | "right" | "bottom";
}

/** Y-axis options for cartesian charts (line/area/bar/scatter). */
export interface AxisOptions {
  scale?: "linear" | "log";
  /** Soft bounds: applied only if the data doesn't already exceed them
   *  (ECharts `min`/`max`), so outliers still show. */
  softMin?: number;
  softMax?: number;
  label?: string;
}

/** Chart-chrome options that aren't per-field: legend + axes. Optional
 *  and additive — absence preserves the prior auto behaviour. */
export interface PanelOptions {
  legend?: LegendOptions;
  yAxis?: AxisOptions;
}

/** A client-side transform applied to query rows before render. The
 *  discriminated `kind` selects the operation; each carries only the
 *  config that operation needs. Transforms run as an ordered pipeline
 *  (`features/canvas/transforms`) and are pure functions over the row
 *  set — they never fetch. */
export type Transform =
  | { kind: "rename"; from: string; to: string }
  | { kind: "calculated"; field: string; left: string; op: "+" | "-" | "*" | "/"; right: string }
  | { kind: "filter"; field: string; op: "=" | "!=" | ">" | ">=" | "<" | "<="; value: string }
  | {
      kind: "groupBy";
      by: string;
      field: string;
      agg: "sum" | "avg" | "min" | "max" | "count";
      as: string;
    }
  | { kind: "reduce"; field: string; calc: "last" | "first" | "sum" | "avg" | "min" | "max" | "count"; as: string }
  | { kind: "organize"; order: ReadonlyArray<string> };

/** Optional live binding: subscribe to an SSE stream that pushes new
 *  rows for this panel (F5 auth is the transport's concern, not here). */
export interface LiveBinding {
  streamId: string;
}

export interface WidgetConfig {
  query: PanelQuery;
  fields: FieldMapping;
  /** Legacy single warn/crit gauge bounds. Kept for back-compat: panels
   *  authored before the field-config editor still read this. New edits
   *  write {@link FieldConfig.defaults.thresholds} instead, and the
   *  resolver (`features/widgets/fieldConfig.ts`) reads `fieldConfig`
   *  first, falling back to these flat fields. */
  thresholds?: Thresholds;
  min?: number;
  max?: number;
  decimals?: number;
  /** Grafana-style per-field display config + overrides. Additive and
   *  optional; absence means "render exactly as before". */
  fieldConfig?: FieldConfig;
  /** Chart-chrome options (legend, axes). Additive and optional. */
  options?: PanelOptions;
  /** Ordered client-side transform pipeline applied to query rows before
   *  render. Absence (or empty) means rows pass through untouched. */
  transforms?: ReadonlyArray<Transform>;
  live?: LiveBinding;
}

export interface Widget {
  id: string;
  type: WidgetType;
  title: string;
  subtitle?: string;
  layout: WidgetLayout;
  config: WidgetConfig;
}

export interface Dashboard {
  id: string;
  name: string;
  slug: string;
  description?: string;
  /** lucide icon name */
  icon: string;
  /** accent hsl, e.g. "152 76% 44%" */
  accent: string;
  starred?: boolean;
  widgets: ReadonlyArray<Widget>;
  updatedAt: string;
}

/** The kinds of dashboard variable (WS-02), mirroring the wire
 *  `VariableKind`. Each kind sources its option list differently; the
 *  resolver (`features/variables/resolve.ts`) populates options per kind
 *  and the binder only ever receives the *resolved* string values. */
export type VariableKind =
  | "constant"
  | "custom"
  | "query"
  | "datasource"
  | "interval"
  | "textbox"
  | "context";

/** The four named sources a page's `context` is assembled from (WS-13 §1),
 *  kept separate (not pre-flattened) so a `context` variable can address
 *  exactly one and the precedence is explicit. A `context` variable's config
 *  names one source + a key; resolution is synchronous (no fetch). */
export type ContextSource = "nav" | "url" | "tag" | "values";

/** The resolved view of a page's place at render time (WS-13 §1) — read-only
 *  input to variable resolution, never a fourth persistence store. Assembled
 *  in `features/variables/context.ts` from the nav node, the URL, the
 *  dashboard's tags, and the nav node's `values` override. */
export interface PageContext {
  /** The nav node the page was opened under, if any. */
  nav?: {
    nodeId: string;
    slug: string;
    name: string;
    /** Ancestor titles, root-first, for `nav` + `path[n]`. */
    path: string[];
  };
  /** URL query params — both `var-*` (WS-02) and bare (`?building=b1`). */
  url: Record<string, string | string[]>;
  /** This dashboard's tags (key → value|null), with nav `context.tags`
   *  merged over them. */
  tags: Record<string, string | null>;
  /** The nav node's `context.values` — explicit per-mount overrides. */
  values: Record<string, string | string[]>;
}

/** One selectable option in a variable's list: a display `text` and the
 *  `value` bound into the query. For most kinds text === value; a query
 *  variable can project a separate `__text` column for the label. */
export interface VariableOption {
  text: string;
  value: string;
}

/** A resolved variable, as the bar renders it and the query layer reads
 *  it: the definition plus its computed option list and current
 *  selection. The selection is always an array (single-select has one
 *  entry) so multi/All expansion is uniform downstream. */
export interface ResolvedVariable {
  id: string;
  name: string;
  label?: string;
  kind: VariableKind;
  options: ReadonlyArray<VariableOption>;
  /** The raw, kind-specific authoring config (opaque). Carried through so
   *  the editor can reseed its fields and the dependency pass can read a
   *  query variable's SQL without a refetch. */
  optionsConfig: unknown;
  /** Currently selected value(s). Multi/All expands to several. */
  current: ReadonlyArray<string>;
  multi: boolean;
  includeAll: boolean;
  hidden: boolean;
  sortOrder: number;
}

/** One result row, keyed by query-result column name. The widget reads
 *  the columns named in its field mapping (`x` for the axis, each
 *  series' `value`). This is the raw `POST /query` row shape — no widget
 *  vocabulary leaks into it. */
export type SeriesPoint = Record<string, string | number | null | undefined>;

/** The data a widget renders from — pure rows, already fetched and
 *  reshaped. Widgets receive this via props; they never fetch (F6). */
export interface WidgetData {
  points: ReadonlyArray<SeriesPoint>;
}
