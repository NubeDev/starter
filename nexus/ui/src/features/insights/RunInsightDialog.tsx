import { useState, type FormEvent } from "react";
import { useMutation } from "@tanstack/react-query";
import { Play } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";

import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import { previewInsight } from "@/api/insights/preview";
import type { InsightSummary, QueryResponse } from "@/api/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { ResultGrid } from "@/features/query-editor/ResultGrid";
import { SqlEditor } from "@/features/sql-editor";

// Same cap the Workbench uses — keep the preview payload small.
const SAMPLE_CAP = 500;

// A starter query so Run is one click for the common telemetry case. The user
// can edit it (or point at any datasource) before running.
const DEFAULT_SQL =
  'SELECT "timestamp" AS time, value FROM telemetry_raw ORDER BY "timestamp" LIMIT 100';

type RunResult =
  | { kind: "ok"; result: QueryResponse; rowsIn: number }
  | { kind: "script-error"; errKind: string; message: string };

// Quick-run an insight straight from the list — no need to open the Workbench
// just to sanity-check what a transform does. Pick a datasource, run a small
// sample query, and the saved script is applied to those rows via
// `/insights/preview` (rows-in / rows-out, nothing saved). A script error comes
// back as `ok:false` (HTTP 200) and is shown inline; a transport failure throws
// and surfaces as the mutation error.
export function RunInsightDialog({
  insight,
  onClose,
}: {
  insight: InsightSummary | null;
  onClose: () => void;
}) {
  const client = useStarterClient();
  const [datasourceId, setDatasourceId] = useState<string | undefined>();
  const [sql, setSql] = useState(DEFAULT_SQL);

  // One round-trip: query the datasource for a sample, then preview the insight
  // over those rows. Resolves to a RunResult so a script error is a normal
  // (non-throwing) outcome the dialog renders inline.
  const run = useMutation<RunResult, Error, void>({
    mutationFn: async () => {
      if (!insight) throw new Error("No insight selected.");
      const sample = datasourceId
        ? await queryDatasource(client, datasourceId, { sql })
        : await runQuery(client, { sql });
      const rows =
        sample.rows.length > SAMPLE_CAP
          ? sample.rows.slice(0, SAMPLE_CAP)
          : sample.rows;
      const res = await previewInsight(client, {
        script: insight.script,
        rows,
      });
      if (res.ok === true && "result" in res) {
        return { kind: "ok", result: res.result, rowsIn: res.row_count_in };
      }
      const err = (res as { error: { kind: string; message: string } }).error;
      return { kind: "script-error", errKind: err.kind, message: err.message };
    },
  });

  // Reset transient run state when the dialog closes so reopening is clean.
  function close() {
    run.reset();
    onClose();
  }

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    run.mutate();
  }

  const canRun = sql.trim().length > 0 && !!datasourceId && !run.isPending;
  const result = run.data;

  return (
    <Dialog open={insight !== null} onOpenChange={(o) => !o && close()}>
      <DialogContent className="glass flex max-h-[85vh] w-[min(56rem,92vw)] max-w-none flex-col gap-3">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            Run insight
            <code className="rounded bg-muted/60 px-1.5 py-0.5 font-mono text-xs text-foreground">
              {insight?.name}
            </code>
          </DialogTitle>
          <DialogDescription>
            Apply this transform to a quick sample — pick a datasource, run a
            query, and see the result. Nothing is saved.
          </DialogDescription>
        </DialogHeader>

        <form className="flex min-h-0 flex-col gap-3" onSubmit={onSubmit}>
          <code className="block truncate rounded-md border border-border/60 bg-background/40 px-2 py-1.5 font-mono text-xs text-muted-foreground">
            {insight?.script}
          </code>

          <DatasourcePicker value={datasourceId} onChange={setDatasourceId} />

          <SqlEditor
            value={sql}
            onChange={setSql}
            datasourceId={datasourceId}
            minHeight="5rem"
            ariaLabel="Sample query SQL"
          />

          <Button type="submit" className="gap-2" disabled={!canRun}>
            <Play className="size-4" />
            {run.isPending ? "Running…" : "Run"}
          </Button>
        </form>

        <div className="min-h-0 flex-1 overflow-auto">
          {run.isPending ? (
            <p className="px-1 pt-2 text-xs text-muted-foreground">Running…</p>
          ) : run.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {run.error.message}
            </p>
          ) : result?.kind === "script-error" ? (
            <div
              role="alert"
              className="space-y-1 rounded-md border border-destructive/40 bg-destructive/10 p-3"
            >
              <p className="text-sm font-medium text-destructive">
                Script error ({result.errKind})
              </p>
              <pre className="scrollbar-thin overflow-auto font-mono text-xs text-destructive/90">
                {result.message}
              </pre>
            </div>
          ) : result?.kind === "ok" ? (
            <div className="flex h-full flex-col gap-1.5">
              <span className="text-xs text-muted-foreground">
                {result.rowsIn} row{result.rowsIn === 1 ? "" : "s"} in →{" "}
                {result.result.rows.length} out
              </span>
              <div className="min-h-0 flex-1">
                <ResultGrid result={result.result} />
              </div>
            </div>
          ) : (
            <p className="px-1 pt-2 text-xs text-muted-foreground">
              Pick a datasource and run to preview the transform.
            </p>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
