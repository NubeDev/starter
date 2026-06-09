import type {
  CreatePanelRequest,
  PanelDetail,
  UpdatePanelRequest,
} from "@/api/types";
import type { FieldMapping, Widget, WidgetLayout } from "@/data/types";
import { toWidgetType } from "@/features/widgets/catalog";

// The backend persists a panel as title + sql + datasource_id + viz + an
// *opaque* `layout` JSON it doesn't interpret. The UI's `Widget` needs
// more — grid position *and* a field mapping (which column is the x axis,
// which are series). Since `layout` is opaque, both ride inside it. This
// module is the single boundary where the wire panel and the UI widget
// meet; nothing else reaches into `layout`'s shape.

// `viz` coercion (free wire string → known widget type, with aliases and
// a safe `table` fallback) lives in the widget catalog, so the type list
// and its wire aliases stay in one place. See `toWidgetType`.

const DEFAULT_LAYOUT: WidgetLayout = { x: 0, y: 0, w: 4, h: 4 };

// Shape we write into the opaque `layout` slot.
interface StashedLayout extends WidgetLayout {
  fields?: FieldMapping;
}

function readLayout(layout: unknown): {
  position: WidgetLayout;
  fields: FieldMapping;
} {
  const l = (layout ?? {}) as Partial<StashedLayout>;
  const position: WidgetLayout = {
    x: typeof l.x === "number" ? l.x : DEFAULT_LAYOUT.x,
    y: typeof l.y === "number" ? l.y : DEFAULT_LAYOUT.y,
    w: typeof l.w === "number" ? l.w : DEFAULT_LAYOUT.w,
    h: typeof l.h === "number" ? l.h : DEFAULT_LAYOUT.h,
  };
  // No stored mapping → an empty series list. A panel with no fields
  // renders its empty state; we never invent columns (F0).
  const fields: FieldMapping = l.fields ?? { series: [] };
  return { position, fields };
}

export function panelToWidget(panel: PanelDetail): Widget {
  const { position, fields } = readLayout(panel.layout);
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

// The opaque `layout` payload: grid position + the field mapping the
// backend doesn't model. Used by both create and the layout PATCH so the
// stashed shape stays in one place.
function stashLayout(widget: Widget): StashedLayout {
  return { ...widget.layout, fields: widget.config.fields };
}
