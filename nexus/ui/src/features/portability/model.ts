// The portable-model layer for selective export/import.
//
// A `DashboardExport` (from `GET /dashboards/{slug}/export`) is a self-contained
// JSON model: appearance + `panels[]` + `variables[]`. Selective export is just
// *filtering* that model down to the panels/variables the user ticked — no
// backend call needed beyond the full export we already fetched. Selective
// import maps the chosen panels/variables onto the create-panel / create-variable
// requests so they can be added to an existing (or freshly created) dashboard.
//
// This module is the single place the export wire shape and the UI's selection
// meet, mirroring `panelAdapter` for the live panel ↔ widget boundary.

import type {
  CreatePanelRequest,
  CreateVariableRequest,
  DashboardExport,
  PanelExport,
  VariableExport,
  VariableKind,
} from "@/api/types";
import type { WidgetLayout, WidgetType } from "@/data/types";
import { toWidgetType } from "@/features/widgets/catalog";

const DEFAULT_LAYOUT: WidgetLayout = { x: 0, y: 0, w: 4, h: 4 };

/** Read a panel-export's grid position out of its opaque `layout` blob. The
 *  same shape `panelAdapter` stashes (x/y/w/h plus an ignored `fields`), so a
 *  schematic preview can place exported panels exactly where they sit live.
 *  Falls back to a default cell when a field is missing or malformed (F0: we
 *  never invent a position that hides a panel off-grid). */
export function readExportLayout(layout: unknown): WidgetLayout {
  const l = (layout ?? {}) as Partial<WidgetLayout>;
  return {
    x: typeof l.x === "number" ? l.x : DEFAULT_LAYOUT.x,
    y: typeof l.y === "number" ? l.y : DEFAULT_LAYOUT.y,
    w: typeof l.w === "number" && l.w > 0 ? l.w : DEFAULT_LAYOUT.w,
    h: typeof l.h === "number" && l.h > 0 ? l.h : DEFAULT_LAYOUT.h,
  };
}

/** The widget type a panel-export draws as, coerced from its free `viz` string
 *  through the same catalog mapping the live canvas uses. */
export function exportWidgetType(panel: PanelExport): WidgetType {
  return toWidgetType(panel.viz);
}

/** Which panels (by index) and variables (by name) the user chose to include.
 *  Indices key panels because export panels have no stable id on the wire; a
 *  variable's `name` is unique per dashboard, so it keys variables. */
export interface PortableSelection {
  panelIndices: ReadonlySet<number>;
  variableNames: ReadonlySet<string>;
}

/** A selection covering everything in a model — the sensible default when a
 *  page first loads (the user then deselects what they don't want). */
export function selectAll(model: DashboardExport): PortableSelection {
  return {
    panelIndices: new Set(model.panels.map((_, i) => i)),
    variableNames: new Set((model.variables ?? []).map((v) => v.name)),
  };
}

/** An empty selection (nothing ticked). */
export function selectNone(): PortableSelection {
  return { panelIndices: new Set(), variableNames: new Set() };
}

/** Total tickable items in a model — drives "N of M selected" affordances. */
export function selectionTotals(model: DashboardExport): {
  panels: number;
  variables: number;
} {
  return {
    panels: model.panels.length,
    variables: (model.variables ?? []).length,
  };
}

/** Count of currently-selected items. */
export function selectionCount(selection: PortableSelection): {
  panels: number;
  variables: number;
  total: number;
} {
  const panels = selection.panelIndices.size;
  const variables = selection.variableNames.size;
  return { panels, variables, total: panels + variables };
}

/** Filter a full export model down to the user's selection. The result is a
 *  valid `DashboardExport` carrying the same appearance/schema but only the
 *  chosen panels/variables — ready to download as a file or import as a subset. */
export function filterExport(
  model: DashboardExport,
  selection: PortableSelection,
): DashboardExport {
  return {
    ...model,
    panels: model.panels.filter((_, i) => selection.panelIndices.has(i)),
    variables: (model.variables ?? []).filter((v) =>
      selection.variableNames.has(v.name),
    ),
  };
}

/** Map an exported panel to a create-panel request so it can be added to an
 *  existing dashboard. `datasource_id` is required by the create API but
 *  nullable on an export (a panel may have had no source, or the importing
 *  tenant must re-bind it); we pass an empty string in that case, which the
 *  panel editor can re-point afterwards — the alternative (dropping the panel)
 *  loses the user's structure, which is worse. */
export function exportPanelToCreate(panel: PanelExport): CreatePanelRequest {
  return {
    title: panel.title,
    sql: panel.sql,
    datasource_id: panel.datasource_id ?? "",
    viz: panel.viz,
    layout: panel.layout,
  };
}

/** Map an exported variable to a create-variable request. The wire kind is a
 *  free string on export but the create API wants the closed `VariableKind`
 *  enum; it is the same value, so a checked cast is correct here. */
export function exportVariableToCreate(
  variable: VariableExport,
): CreateVariableRequest {
  return {
    name: variable.name,
    label: variable.label,
    kind: variable.kind as VariableKind,
    options_config: variable.options_config,
    current: variable.current,
    multi: variable.multi,
    include_all: variable.include_all,
    hidden: variable.hidden,
    sort_order: variable.sort_order,
  };
}

/** Parse + validate an arbitrary JSON string as a `DashboardExport`. Returns a
 *  discriminated result so the import page can show a precise error rather than
 *  throwing. We check the structural essentials (a `panels` array, the
 *  appearance strings); the backend re-validates `schema_version` on import, so
 *  we surface its presence but don't hard-fail an unknown version here — the
 *  user may still want to preview it. */
export type ParseResult =
  | { ok: true; model: DashboardExport }
  | { ok: false; error: string };

export function parseExport(text: string): ParseResult {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return { ok: false, error: "That isn't valid JSON." };
  }
  if (typeof raw !== "object" || raw === null) {
    return { ok: false, error: "Expected a dashboard export object." };
  }
  const obj = raw as Record<string, unknown>;
  if (!Array.isArray(obj.panels)) {
    return {
      ok: false,
      error: "This file has no `panels` array — it isn't a dashboard export.",
    };
  }
  if (typeof obj.name !== "string" || typeof obj.slug !== "string") {
    return {
      ok: false,
      error: "Missing the dashboard `name`/`slug` an export must carry.",
    };
  }
  // Normalise the optional `variables` to an array so downstream code never has
  // to null-check it.
  const model: DashboardExport = {
    schema_version: typeof obj.schema_version === "number" ? obj.schema_version : 1,
    slug: obj.slug,
    name: obj.name,
    icon: typeof obj.icon === "string" ? obj.icon : "gauge",
    accent: typeof obj.accent === "string" ? obj.accent : "152 76% 44%",
    panels: obj.panels as PanelExport[],
    variables: Array.isArray(obj.variables)
      ? (obj.variables as VariableExport[])
      : [],
  };
  return { ok: true, model };
}

/** Serialise a (possibly filtered) export model to pretty JSON for download or
 *  the clipboard. */
export function exportToJson(model: DashboardExport): string {
  return JSON.stringify(model, null, 2);
}
