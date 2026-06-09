import type { Widget, WidgetData } from "@/data/types";
import { WIDGET_RENDERERS } from "@/features/widgets/renderMap";

// Dispatches a widget to its renderer by type via the registry. The
// registry (`WIDGET_RENDERERS`, keyed by `WidgetType`) is the one place
// panel types map to renderers; adding a type is a compile error there
// until it's handled. Pure — every renderer takes the same typed props
// and fetches nothing (F6).
export function RenderWidget({
  widget,
  data,
}: {
  widget: Widget;
  data: WidgetData;
}) {
  return WIDGET_RENDERERS[widget.type]({ widget, data });
}
