import type { Widget } from "@/data/types";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";
import { RenderWidget } from "@/features/widgets/renderWidget";
import { useWidgetQuery } from "@/features/widgets/useWidgetQuery";

// Live preview of the draft panel. Runs the same query a real panel runs
// (`useWidgetQuery`, keyed by datasource + SQL) so editing display config
// or transforms re-renders from the cached rows without refetching — only
// a query change re-runs the request. `RenderWidget` applies the draft's
// transform pipeline, so the preview reflects every tab's edits live.
export function PreviewPane({ widget }: { widget: Widget }) {
  const state = useWidgetQuery(widget);
  return (
    <div className="glass flex h-full min-h-0 flex-col rounded-xl p-3">
      <header className="mb-2 flex items-baseline justify-between gap-2">
        <h3 className="truncate text-sm font-medium text-foreground">
          {widget.title || "Untitled panel"}
        </h3>
        <span className="text-xs uppercase tracking-wide text-muted-foreground">
          Preview
        </span>
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
