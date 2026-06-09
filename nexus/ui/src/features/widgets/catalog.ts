import type { WidgetType } from "@/data/types";

// The single source of truth for *what panel types exist* and the
// per-type metadata the rest of the app needs to enumerate, size, place,
// and author them. Intentionally data-only and React-free (no JSX): the
// API boundary (`panelAdapter`) and the grid layout both read it, and
// neither may pull the chart library or React into their layer. The
// renderers (JSX) are layered on top in `renderMap.tsx`, keyed by the
// same `WidgetType`, so the catalog stays the one place a type is
// declared and `renderMap` is a compile error until it handles it.
//
// Adding a panel type is: one entry here + one builder + one renderer in
// `renderMap`. The four tables that used to be edited in lockstep
// (render switch, MIN_SIZE, AddWidgetDialog SIZE/TYPES/NEEDS_X,
// panelAdapter WIDGET_TYPES) all derive from this object.

/** Whether a panel needs the x (category/time) column and how many
 *  series it draws. The add-panel form reads `x` to decide whether to
 *  prompt for an x column; `series` documents intent for the future
 *  multi-series editor. */
export interface FieldRoles {
  /** `"required"` → the form prompts for an x column; `"none"` →
   *  single-value panels (stat/gauge) that read only the first series. */
  x: "required" | "none";
  series: "single" | "multi";
}

export interface WidgetDescriptor {
  type: WidgetType;
  /** Human label for the add-panel picker (the raw type is lowercase). */
  label: string;
  /** lucide icon name, for the picker. */
  icon: string;
  /** Default footprint when a panel of this type is first added. */
  defaultSize: { w: number; h: number };
  /** Minimum grid footprint enforced during resize. */
  minSize: { minW: number; minH: number };
  roles: FieldRoles;
}

// Keyed by WidgetType so the compiler forces an entry for every type —
// adding to the union without describing it here fails to typecheck.
export const WIDGET_CATALOG: Record<WidgetType, WidgetDescriptor> = {
  line: {
    type: "line",
    label: "Line",
    icon: "TrendingUp",
    defaultSize: { w: 6, h: 4 },
    minSize: { minW: 3, minH: 3 },
    roles: { x: "required", series: "multi" },
  },
  area: {
    type: "area",
    label: "Area",
    icon: "AreaChart",
    defaultSize: { w: 6, h: 4 },
    minSize: { minW: 3, minH: 3 },
    roles: { x: "required", series: "multi" },
  },
  bar: {
    type: "bar",
    label: "Bar",
    icon: "BarChart3",
    defaultSize: { w: 6, h: 4 },
    minSize: { minW: 3, minH: 3 },
    roles: { x: "required", series: "multi" },
  },
  scatter: {
    type: "scatter",
    label: "Scatter",
    icon: "ScatterChart",
    defaultSize: { w: 6, h: 4 },
    minSize: { minW: 3, minH: 3 },
    roles: { x: "required", series: "multi" },
  },
  heatmap: {
    type: "heatmap",
    label: "Heatmap",
    icon: "Grid3x3",
    defaultSize: { w: 6, h: 4 },
    minSize: { minW: 4, minH: 3 },
    roles: { x: "required", series: "multi" },
  },
  pie: {
    type: "pie",
    label: "Pie",
    icon: "PieChart",
    defaultSize: { w: 4, h: 4 },
    minSize: { minW: 3, minH: 3 },
    roles: { x: "required", series: "single" },
  },
  gauge: {
    type: "gauge",
    label: "Gauge",
    icon: "Gauge",
    defaultSize: { w: 3, h: 3 },
    minSize: { minW: 2, minH: 3 },
    roles: { x: "none", series: "single" },
  },
  stat: {
    type: "stat",
    label: "Stat",
    icon: "Hash",
    defaultSize: { w: 3, h: 2 },
    minSize: { minW: 2, minH: 2 },
    roles: { x: "none", series: "single" },
  },
  status: {
    type: "status",
    label: "Status",
    icon: "CircleDot",
    defaultSize: { w: 3, h: 4 },
    minSize: { minW: 3, minH: 3 },
    roles: { x: "required", series: "single" },
  },
  table: {
    type: "table",
    label: "Table",
    icon: "Table",
    defaultSize: { w: 6, h: 4 },
    minSize: { minW: 4, minH: 4 },
    roles: { x: "required", series: "multi" },
  },
};

/** Every known widget type. Derived from the catalog so it can never
 *  drift from the renderers/descriptors. */
export const WIDGET_TYPES = Object.keys(WIDGET_CATALOG) as WidgetType[];

const TYPE_SET: ReadonlySet<string> = new Set(WIDGET_TYPES);

/** Narrow a free-string `viz` from the wire to a known widget type,
 *  honouring a few aliases the backend emits, and falling back to
 *  `table` for anything unrecognised rather than crashing the canvas. */
export function toWidgetType(viz: string | null | undefined): WidgetType {
  if (!viz) return "table";
  const alias = VIZ_ALIASES[viz];
  if (alias) return alias;
  return TYPE_SET.has(viz) ? (viz as WidgetType) : "table";
}

// Backend `viz` strings that don't map 1:1 to a widget type. Kept here so
// the wire-vocabulary mapping lives next to the type list it maps onto.
const VIZ_ALIASES: Record<string, WidgetType> = {
  donut: "pie",
  column: "bar",
};
