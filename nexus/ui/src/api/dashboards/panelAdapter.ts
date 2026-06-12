import type {
  CreatePanelRequest,
  PanelDetail,
  UpdatePanelRequest,
} from "@/api/types";
import type {
  FieldConfig,
  FieldMapping,
  PanelOptions,
  Thresholds,
  Transform,
  Widget,
  WidgetLayout,
} from "@/data/types";
import { toWidgetType } from "@/features/widgets/catalog";

// The backend persists a panel as title + sql + datasource_id + viz + an
// *opaque* `layout` JSON it doesn't interpret. The UI's `Widget` needs
// more — grid position, the field mapping (x + series), and the display
// config the editor authors: `fieldConfig` (unit/decimals/thresholds/
// overrides), `options` (legend/axes), and `transforms`. The backend has
// no column for any of these, so the WHOLE display config rides inside the
// opaque `layout` blob. This module is the single boundary where the wire
// panel and the UI widget meet; nothing else reaches into `layout`'s shape.
//
// Earlier this stashed only `fields`, so every Field/Overrides/Legend/
// Transforms edit was silently dropped on save (it survived in the live
// preview but vanished on reload). Round-tripping the full display config
// fixes that.

// `viz` coercion (free wire string → known widget type, with aliases and
// a safe `table` fallback) lives in the widget catalog, so the type list
// and its wire aliases stay in one place. See `toWidgetType`.

const DEFAULT_LAYOUT: WidgetLayout = { x: 0, y: 0, w: 4, h: 4 };

// The display config the editor owns, persisted alongside the grid in the
// opaque `layout` blob. `query` is NOT here — sql/datasource_id are their
// own backend columns.
interface StashedDisplay {
  fields?: FieldMapping;
  thresholds?: Thresholds;
  min?: number;
  max?: number;
  decimals?: number;
  fieldConfig?: FieldConfig;
  options?: PanelOptions;
  transforms?: ReadonlyArray<Transform>;
  // Kind-mode (WS-10): the wire panel has no `kind` column, so a panel that
  // runs a declarative query-kind instead of raw SQL stashes the kind name +
  // params in the opaque `layout` blob. `useWidgetQuery` reads them back onto
  // the request so the query runs as `POST /api/v1/query { kind, params }` —
  // the only read path that resolves host tokens (`$caller_tenant_id` /
  // `$caller_team_ids`) for a non-admin against the control-plane DB. This is
  // what lets a per-user "My devices" panel render an extension-owned table.
  kind?: string;
  kindParams?: Record<string, string | number | boolean>;
}

// Shape we write into the opaque `layout` slot: grid position + display.
type StashedLayout = WidgetLayout & StashedDisplay;

function readLayout(layout: unknown): {
  position: WidgetLayout;
  display: StashedDisplay;
} {
  const l = (layout ?? {}) as Partial<StashedLayout>;
  const position: WidgetLayout = {
    x: typeof l.x === "number" ? l.x : DEFAULT_LAYOUT.x,
    y: typeof l.y === "number" ? l.y : DEFAULT_LAYOUT.y,
    w: typeof l.w === "number" ? l.w : DEFAULT_LAYOUT.w,
    h: typeof l.h === "number" ? l.h : DEFAULT_LAYOUT.h,
  };
  // No stored mapping → an empty series list. A panel with no fields
  // renders its empty state; we never invent columns (F0). The rest of the
  // display config is optional — absence means "render as before".
  const display: StashedDisplay = {
    fields: l.fields ?? { series: [] },
    thresholds: l.thresholds,
    min: l.min,
    max: l.max,
    decimals: l.decimals,
    fieldConfig: l.fieldConfig,
    options: l.options,
    transforms: l.transforms,
    kind: l.kind,
    kindParams: l.kindParams,
  };
  return { position, display };
}

// Pack a widget's full display config (everything except `query`) into the
// opaque `layout` slot, dropping undefined keys so the blob stays clean.
function stashLayout(widget: Widget): StashedLayout {
  const c = widget.config;
  const stashed: StashedLayout = { ...widget.layout, fields: c.fields };
  if (c.thresholds !== undefined) stashed.thresholds = c.thresholds;
  if (c.min !== undefined) stashed.min = c.min;
  if (c.max !== undefined) stashed.max = c.max;
  if (c.decimals !== undefined) stashed.decimals = c.decimals;
  if (c.fieldConfig !== undefined) stashed.fieldConfig = c.fieldConfig;
  if (c.options !== undefined) stashed.options = c.options;
  if (c.transforms !== undefined) stashed.transforms = c.transforms;
  // Round-trip the kind-mode query through the opaque blob (see StashedDisplay).
  if (c.query.kind !== undefined) stashed.kind = c.query.kind;
  if (c.query.kindParams !== undefined) stashed.kindParams = c.query.kindParams;
  return stashed;
}

export function panelToWidget(panel: PanelDetail): Widget {
  const { position, display } = readLayout(panel.layout);
  const { fields = { series: [] }, kind, kindParams, ...rest } = display;
  return {
    id: panel.id,
    type: toWidgetType(panel.viz),
    title: panel.title,
    layout: position,
    config: {
      query: {
        datasourceId: panel.datasource_id ?? "",
        sql: panel.sql,
        // Kind-mode lifted out of the opaque layout blob (see StashedDisplay).
        ...(kind ? { kind } : {}),
        ...(kindParams ? { kindParams } : {}),
        // RW-06: an attached insight rides on the query as its own columns, not
        // inside the opaque layout blob. `?? undefined` so a missing/`null` id
        // becomes "no insight" rather than an empty string.
        ...(panel.insight_id ? { insightId: panel.insight_id } : {}),
        ...(panel.insight_params !== undefined &&
        panel.insight_params !== null
          ? { insightParams: panel.insight_params }
          : {}),
      },
      fields,
      // Spread the rest of the round-tripped display config (fieldConfig,
      // options, transforms, legacy thresholds/min/max/decimals); each is
      // omitted from the blob when unset, so undefined keys don't appear.
      ...rest,
    },
  };
}

// Pack a widget into the create-panel body, stashing position + field
// mapping in the opaque `layout`. The backend echoes `layout` back
// untouched, so `panelToWidget` reconstructs the full widget.
export function widgetToCreatePanel(widget: Widget): CreatePanelRequest {
  const q = widget.config.query;
  return {
    title: widget.title,
    sql: q.sql,
    datasource_id: q.datasourceId,
    viz: widget.type,
    layout: stashLayout(widget),
    // Attach an insight only when one is set; omit otherwise (create has no
    // "detach" — absence is "none").
    ...(q.insightId ? { insight_id: q.insightId } : {}),
    ...(q.insightId && q.insightParams !== undefined
      ? { insight_params: q.insightParams }
      : {}),
  };
}

// A layout-only PATCH for a moved/resized panel: re-stash the grid
// position with the (unchanged) field mapping so the opaque layout stays
// complete, and send nothing else — the panel's sql/viz/datasource are
// untouched by a drag.
export function widgetToLayoutPatch(widget: Widget): UpdatePanelRequest {
  return { layout: stashLayout(widget) };
}

// A full PATCH for an edited panel (the properties panel): title, query,
// viz *and* the re-stashed layout (so an edited field mapping persists in
// the opaque blob). Mirrors `widgetToCreatePanel` but for an existing id;
// the position inside `layout` is the widget's current one, unchanged by a
// properties edit.
export function widgetToUpdatePanel(widget: Widget): UpdatePanelRequest {
  const q = widget.config.query;
  return {
    title: widget.title,
    sql: q.sql,
    datasource_id: q.datasourceId,
    viz: widget.type,
    layout: stashLayout(widget),
    // A full panel save expresses the editor's current state, so the insight is
    // either set or explicitly detached — never "leave unchanged". The backend
    // detach intent rides a `clear_insight` flag (not a wire `null`, which serde
    // can't tell from "absent"), mirroring dashboards' `clear_folder`.
    ...(q.insightId
      ? {
          insight_id: q.insightId,
          ...(q.insightParams !== undefined
            ? { insight_params: q.insightParams }
            : {}),
        }
      : { clear_insight: true }),
  };
}
