import { useMemo } from "react";
import { AlertTriangle, BarChart3, TableIcon } from "lucide-react";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@nube/starter-ui-kit/components/tabs";

import type { QueryResponse } from "@/api/types";
import { ResultGrid } from "@/features/query-editor/ResultGrid";
import { EChart } from "@/features/widgets/EChart";
import { Empty } from "@/features/state/Empty";
import { Loading } from "@/features/state/Loading";
import {
  buildResultChartOption,
  isChartable,
} from "@/features/insights/resultChartOption";
import type { PreviewState } from "@/features/insights/usePreviewInsight";

// The live result of the transform. This pane carries the most important
// feedback in the feature: a script error renders as an inline alert (NOT a
// thrown error, NOT an empty grid), and a success renders the transformed rows
// with a row-delta header and a before/after view of the input sample.
export function ResultPane({
  preview,
  sample,
  hasSample,
}: {
  preview: PreviewState;
  sample: QueryResponse | null;
  hasSample: boolean;
}) {
  return (
    <div className="glass flex h-full min-h-0 flex-col gap-3 rounded-xl p-3">
      <div className="flex items-center gap-2">
        <h3 className="text-sm font-semibold">Result</h3>
        <PreviewHeader preview={preview} />
      </div>

      <div className="min-h-0 flex-1">
        {!hasSample ? (
          <Empty
            title="No sample yet"
            description="Run a query in the Source pane to load rows, then the result updates live as you edit the transform."
          />
        ) : preview.status === "idle" ? (
          <Empty
            title="Write a transform"
            description="Type a Rhai transform in the middle pane — the result previews here automatically."
          />
        ) : preview.status === "loading" ? (
          <Loading label="Previewing…" />
        ) : preview.status === "transport-error" ? (
          <div role="alert" className="rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
            {preview.message}
          </div>
        ) : preview.status === "script-error" ? (
          <div
            role="alert"
            className="flex flex-col gap-1.5 rounded-md border border-destructive/50 bg-destructive/10 p-3"
          >
            <div className="flex items-center gap-2 text-sm font-medium text-destructive">
              <AlertTriangle className="size-4" />
              {kindLabel(preview.kind)} error
            </div>
            <pre className="scrollbar-thin overflow-auto whitespace-pre-wrap font-mono text-xs text-destructive/90">
              {preview.message}
            </pre>
          </div>
        ) : (
          <OutputView result={preview.result} sample={sample} />
        )}
      </div>
    </div>
  );
}

// The "in → out · ms" strip plus a kind badge, shown whenever there's a result
// or an error so the user always knows what the last run did.
function PreviewHeader({ preview }: { preview: PreviewState }) {
  if (preview.status === "ok") {
    const { result, rowCountIn } = preview;
    return (
      <div className="ms-auto flex items-center gap-2 text-xs text-muted-foreground">
        <span className="tabular">
          {rowCountIn} → {result.stats.row_count} rows
        </span>
        <span>·</span>
        <span className="tabular">{result.stats.elapsed_ms}ms</span>
        {result.stats.truncated ? (
          <span className="rounded-full bg-amber-500/15 px-1.5 text-amber-600 dark:text-amber-400">
            capped
          </span>
        ) : null}
      </div>
    );
  }
  if (preview.status === "script-error") {
    return (
      <span className="ms-auto rounded-full bg-destructive/15 px-2 py-0.5 text-xs font-medium text-destructive">
        {kindLabel(preview.kind)}
      </span>
    );
  }
  return null;
}

// Success body: tabbed Table / Chart of the output, with a compact before/after
// of the input sample so the user sees what the transform changed.
function OutputView({
  result,
  sample,
}: {
  result: QueryResponse;
  sample: QueryResponse | null;
}) {
  const option = useMemo(() => buildResultChartOption(result), [result]);
  const chartable = isChartable(result);

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <Tabs defaultValue="table" className="flex min-h-0 flex-1 flex-col">
        <TabsList>
          <TabsTrigger value="table" className="gap-1.5">
            <TableIcon className="size-3.5" />
            Table
          </TabsTrigger>
          <TabsTrigger value="chart" className="gap-1.5">
            <BarChart3 className="size-3.5" />
            Chart
          </TabsTrigger>
        </TabsList>
        <TabsContent value="table" className="min-h-0 flex-1">
          <ResultGrid result={result} />
        </TabsContent>
        <TabsContent value="chart" className="min-h-0 flex-1">
          {chartable ? (
            <EChart option={option} ariaLabel="Transform result chart" />
          ) : (
            <Empty
              title="Nothing to chart"
              description="The result has no numeric columns to plot."
            />
          )}
        </TabsContent>
      </Tabs>

      {sample ? (
        <details className="rounded-md border border-border/60">
          <summary className="cursor-pointer select-none px-3 py-1.5 text-xs font-medium text-muted-foreground">
            Before / after — input sample ({sample.rows.length} rows)
          </summary>
          <div className="max-h-40 overflow-auto border-t border-border/60">
            <ResultGrid result={sample} />
          </div>
        </details>
      ) : null}
    </div>
  );
}

function kindLabel(kind: string): string {
  if (kind === "compile") return "Compile";
  if (kind === "runtime") return "Runtime";
  if (kind === "limit") return "Limit";
  return kind;
}
