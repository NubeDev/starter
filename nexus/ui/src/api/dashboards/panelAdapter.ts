import type {
  CreatePanelRequest,
  PanelDetail,
} from "@/api/types";
import type {
  FieldMapping,
  Widget,
  WidgetLayout,
  WidgetType,
} from "@/data/types";

// The backend persists a panel as title + sql + datasource_id + viz + an
// *opaque* `layout` JSON it doesn't interpret. The UI's `Widget` needs
// more — grid position *and* a field mapping (which column is the x axis,
// which are series). Since `layout` is opaque, both ride inside it. This
// module is the single boundary where the wire panel and the UI widget
// meet; nothing else reaches into `layout`'s shape.

const WIDGET_TYPES: ReadonlyArray<WidgetType> = [
  "line",
  "area",
  "gauge",
  "stat",
  "status",
  "table",
];

// `viz` is a free string on the wire (`line | bar | table | …`); coerce to
// a known widget type, mapping `bar`→`table` and anything unknown to a
// safe `table` default rather than crashing the canvas.
function toWidgetType(viz: string | null | undefined): WidgetType {
  if (viz && (WIDGET_TYPES as ReadonlyArray<string>).includes(viz)) {
    return viz as WidgetType;
  }
  return "table";
}

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
  const layout: StashedLayout = {
    ...widget.layout,
    fields: widget.config.fields,
  };
  return {
    title: widget.title,
    sql: widget.config.query.sql,
    datasource_id: widget.config.query.datasourceId,
    viz: widget.type,
    layout,
  };
}
