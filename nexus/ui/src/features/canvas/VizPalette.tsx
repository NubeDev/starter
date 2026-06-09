import type { WidgetType } from "@/data/types";
import { WIDGET_CATALOG, WIDGET_TYPES } from "@/features/widgets/catalog";
import { widgetIcon } from "@/features/widgets/icon";

// The visualization palette: a grid of icon tiles, one per widget type,
// that the user drags onto the canvas to add a panel (Grafana-style).
// Shown in edit mode. The drag itself is plain HTML5 DnD — each tile sets
// the dragged type via `onPick` on drag start; react-grid-layout's
// `isDroppable`/`onDrop` on the canvas turns the drop into a grid cell, so
// no drop-coordinate math lives here. Tiles derive entirely from the
// widget catalog, so a new type appears here for free.
export function VizPalette({
  onPick,
}: {
  /** Called when a tile starts being dragged, with its type. The page
   *  holds this so the grid's `onDrop` knows what was dropped. */
  onPick: (type: WidgetType) => void;
}) {
  return (
    <aside className="glass flex w-44 shrink-0 flex-col gap-2 rounded-xl p-3">
      <h3 className="px-1 text-xs font-medium uppercase tracking-wide text-muted-foreground">
        Add panel
      </h3>
      <p className="px-1 text-xs text-muted-foreground">
        Drag a chart onto the board.
      </p>
      <div className="grid grid-cols-2 gap-2">
        {WIDGET_TYPES.map((type) => {
          const d = WIDGET_CATALOG[type];
          const Icon = widgetIcon(d.icon);
          return (
            <button
              key={type}
              type="button"
              // The browser needs `draggable` to emit drag events; the
              // grid's droppable layer handles the actual placement.
              draggable
              onDragStart={(e) => {
                // react-grid-layout reads `text/plain` off the drag to
                // confirm an external drop; the payload itself is unused
                // (we track the type via `onPick`) but must be set or some
                // browsers cancel the drag.
                e.dataTransfer.setData("text/plain", type);
                e.dataTransfer.effectAllowed = "copy";
                onPick(type);
              }}
              className="flex cursor-grab flex-col items-center gap-1 rounded-lg border border-border/60 bg-card/40 p-2 text-center transition-colors hover:border-primary/60 hover:bg-accent/30 active:cursor-grabbing"
              title={`Drag to add a ${d.label} panel`}
            >
              <Icon className="size-5 text-primary" aria-hidden />
              <span className="text-[11px] text-foreground">{d.label}</span>
            </button>
          );
        })}
      </div>
    </aside>
  );
}
