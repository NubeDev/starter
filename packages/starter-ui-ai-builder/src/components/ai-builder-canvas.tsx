import * as React from "react";
import {
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import {
  Renderer,
  SduiProvider,
  type UiComponentTree,
} from "@nube/starter-sdui-react";
import { cn } from "../lib/utils.js";

export interface AiBuilderCanvasProps
  extends React.HTMLAttributes<HTMLDivElement> {
  tree: UiComponentTree | null;
  /** Rendered when `tree` is null. */
  emptyState?: React.ReactNode;
  /** Optional badge showing how many `patch` events are buffered
   *  (R1 — waiting for parents). Default: hidden. */
  bufferedPatches?: number;
}

const NOOP_QUERY_CLIENT = new QueryClient({
  defaultOptions: { queries: { retry: false } },
});

const NOOP_ACTION = async () => ({ type: "noop" as const });

export const AiBuilderCanvas = React.forwardRef<
  HTMLDivElement,
  AiBuilderCanvasProps
>(({ tree, emptyState, bufferedPatches, className, ...props }, ref) => {
  const [pageState, setPageState] = React.useState<Record<string, unknown>>({});
  const mergePageState = React.useCallback(
    (patch: Record<string, unknown>) =>
      setPageState((prev) => ({ ...prev, ...patch })),
    [],
  );
  const customRegistry = React.useMemo(() => new Map(), []);
  const treeQueryKey = React.useMemo(() => ["ai-builder-canvas"] as const, []);

  return (
    <div
      ref={ref}
      data-slot="ai-builder-canvas"
      className={cn(
        "relative flex h-full min-h-0 w-full flex-col overflow-hidden bg-background",
        className,
      )}
      {...props}
    >
      {bufferedPatches ? (
        <div className="pointer-events-none absolute right-3 top-3 z-10 inline-flex items-center gap-1.5 rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 text-[10px] font-medium text-amber-700 dark:text-amber-300">
          <span className="inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-amber-500" />
          {bufferedPatches} buffered
        </div>
      ) : null}

      <div className="flex min-h-0 flex-1 overflow-auto p-4">
        {tree ? (
          <QueryClientProvider client={NOOP_QUERY_CLIENT}>
            <SduiProvider
              dispatchAction={NOOP_ACTION}
              customRegistry={customRegistry}
              pageState={pageState}
              setPageState={mergePageState}
              treeQueryKey={treeQueryKey}
              writePlan={[]}
            >
              <div className="mx-auto w-full max-w-5xl">
                <Renderer node={tree.root} />
              </div>
            </SduiProvider>
          </QueryClientProvider>
        ) : (
          <div className="m-auto text-sm text-muted-foreground">
            {emptyState ?? "Send a prompt to start building."}
          </div>
        )}
      </div>
    </div>
  );
});
AiBuilderCanvas.displayName = "AiBuilderCanvas";
