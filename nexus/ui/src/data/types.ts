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
  | "table";

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
}

/** One drawn series, mapped from a column in the query result. */
export interface SeriesField {
  /** Result column holding the numeric value. */
  value: string;
  label?: string;
  unit?: string;
  /** hsl string, e.g. "152 76% 44%"; defaults to the chart palette. */
  color?: string;
}

/** Maps query-result columns onto chart roles. The `x` column is the
 *  category/time axis for line/area; omitted for single-value panels
 *  (stat/gauge) that read only the first series. */
export interface FieldMapping {
  x?: string;
  series: ReadonlyArray<SeriesField>;
}

/** Gauge/stat threshold bounds. `crit < warn` encodes a descending
 *  metric (battery SoC), `crit > warn` an ascending one (load). */
export interface Thresholds {
  warn?: number;
  crit?: number;
}

/** Optional live binding: subscribe to an SSE stream that pushes new
 *  rows for this panel (F5 auth is the transport's concern, not here). */
export interface LiveBinding {
  streamId: string;
}

export interface WidgetConfig {
  query: PanelQuery;
  fields: FieldMapping;
  thresholds?: Thresholds;
  min?: number;
  max?: number;
  decimals?: number;
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

/** A single point in time across all mapped series — the shape a widget
 *  receives after the query result is reshaped to its field mapping.
 *  Keyed by series `value` column; `x` carries the axis value. */
export interface SeriesPoint {
  x?: string | number;
  [seriesColumn: string]: string | number | undefined;
}

/** The data a widget renders from — pure rows, already fetched and
 *  reshaped. Widgets receive this via props; they never fetch (F6). */
export interface WidgetData {
  points: ReadonlyArray<SeriesPoint>;
}
