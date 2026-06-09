import type { Widget, WidgetLayout } from "@/data/types";

// Where a newly added panel lands: column 0, on the row just below the
// lowest existing panel. Simple and predictable — the user then drags it
// where they want (once layout-save lands, B5). Avoids overlap without a
// full bin-packing pass.
export function nextSlot(
  widgets: ReadonlyArray<Widget>,
  w: number,
  h: number,
): WidgetLayout {
  const bottom = widgets.reduce(
    (max, widget) => Math.max(max, widget.layout.y + widget.layout.h),
    0,
  );
  return { x: 0, y: bottom, w, h };
}
