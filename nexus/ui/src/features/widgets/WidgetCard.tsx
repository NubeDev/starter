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
// switches on the query outcome.
export function WidgetCard({
  widget,
  state,
}: {
  widget: Widget;
  state: WidgetState;
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
        {widget.subtitle ? (
          <span className="truncate text-xs text-muted-foreground">
            {widget.subtitle}
          </span>
        ) : null}
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
