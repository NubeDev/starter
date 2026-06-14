import type { Widget, WidgetData } from "@/data/types";
import { applyTransforms } from "@/features/canvas/transforms";
import { WIDGET_RENDERERS } from "@/features/widgets/renderMap";

// Dispatches a widget to its renderer by type via the registry. The
// registry (`WIDGET_RENDERERS`, keyed by `WidgetType`) is the one place
// panel types map to renderers; adding a type is a compile error there
// until it's handled. Pure — every renderer takes the same typed props
// and fetches nothing (F6).
//
// The panel's transform pipeline runs here, after fetch and before the
// renderer, so a config-only edit (adding/removing a transform) re-renders
// from the cached rows without re-running the query.
export function RenderWidget({
  widget,
  data,
}: {
  widget: Widget;
  data: WidgetData;
}) {
  const transformed = applyTransforms(data, widget.config.transforms);
  return WIDGET_RENDERERS[widget.type]({ widget, data: transformed });
}
