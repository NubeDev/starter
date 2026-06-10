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
  return stashed;
}

export function panelToWidget(panel: PanelDetail): Widget {
  const { position, display } = readLayout(panel.layout);
  const { fields = { series: [] }, ...rest } = display;
  return {
    id: panel.id,
    type: toWidgetType(panel.viz),
    title: panel.title,
    layout: position,
    config: {
      query: {
        datasourceId: panel.datasource_id ?? "",
        sql: panel.sql,
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
  return {
    title: widget.title,
    sql: widget.config.query.sql,
    datasource_id: widget.config.query.datasourceId,
    viz: widget.type,
    layout: stashLayout(widget),
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
  return {
    title: widget.title,
    sql: widget.config.query.sql,
    datasource_id: widget.config.query.datasourceId,
    viz: widget.type,
    layout: stashLayout(widget),
  };
}
