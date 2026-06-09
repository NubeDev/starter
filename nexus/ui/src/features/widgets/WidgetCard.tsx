import { X } from "lucide-react";

import type { Widget, WidgetData } from "@/data/types";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import { RenderWidget } from "@/features/widgets/renderWidget";

// The query outcome for a panel, passed in by the canvas. The panel
// itself stays pure (F6): the data subscription lives one layer up (the
// canvas runs `POST /query` / the SSE hook per widget and feeds the
// result down), so the card renders loading / error / data without ever
// fetching or fabricating rows (F0).
export type WidgetState =
  | { status: "loading" }
  | { status: "error"; message?: string }
  | { status: "ready"; data: WidgetData };

// Frame around a single panel: glass card, title, and the body that
// switches on the query outcome. In edit mode it shows a remove control;
// `onRemove` is wired only on the canvas (the dashboard owns deletion), so
// a panel rendered elsewhere has no destructive affordance.
export function WidgetCard({
  widget,
  state,
  editing = false,
  onRemove,
}: {
  widget: Widget;
  state: WidgetState;
  editing?: boolean;
  onRemove?: () => void;
}) {
  return (
    <div className="glass card-hover flex h-full flex-col rounded-xl p-3">
      {/* The header is react-grid-layout's drag handle (matched by
          `.widget-drag-handle` in the canvas); the grab cursor only shows
          in edit mode, when the grid makes it draggable. */}
      <header className="widget-drag-handle mb-2 flex items-baseline justify-between gap-2">
        <h3 className="truncate text-sm font-medium text-foreground">
          {widget.title}
        </h3>
        <div className="flex items-center gap-2">
          {widget.subtitle ? (
            <span className="truncate text-xs text-muted-foreground">
              {widget.subtitle}
            </span>
          ) : null}
          {editing && onRemove ? (
            <button
              type="button"
              aria-label={`Remove ${widget.title}`}
              // Stop the click reaching react-grid-layout's drag start.
              onMouseDown={(e) => e.stopPropagation()}
              onClick={onRemove}
              className="rounded p-0.5 text-muted-foreground transition-colors hover:bg-destructive/15 hover:text-destructive"
            >
              <X className="size-4" />
            </button>
          ) : null}
        </div>
      </header>
      <div className="min-h-0 flex-1">
        {state.status === "loading" ? (
          <Loading />
        ) : state.status === "error" ? (
          <ErrorState message={state.message} />
        ) : (
          <RenderWidget widget={widget} data={state.data} />
        )}
      </div>
    </div>
  );
}
