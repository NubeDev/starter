import { useMutation } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { PlayCircle } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import type { QueryRequest, QueryResponse } from "@/api/types";
import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import type { EditorDraft } from "@/features/canvas/PanelEditor/useEditorDraft";
import { AiSqlAssist } from "@/features/ai/AiSqlAssist";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { SqlEditor } from "@/features/sql-editor";
import { useInsights } from "@/features/insights/useInsights";
import { useTimeStore, resolveTimeRange, intervalSecs } from "@/store/time";
import { useVariableStore, toQueryVariables } from "@/store/variables";

// Select sentinel for "no insight" — Radix Select can't hold an empty value.
const NO_INSIGHT = "__none__";

// Query tab: datasource + SQL (the WS-03 CodeMirror editor) + a Test run
// that reports row count / columns / timing without mutating the panel.
// The title also lives here as the first thing a user sets. Edits flow
// into the editor draft; nothing persists until Save.
export function QueryTab({ draft }: { draft: EditorDraft }) {
  const { widget, patch, patchConfig } = draft;
  const datasourceId = widget.config.query.datasourceId || undefined;
  const sql = widget.config.query.sql;
  const insightId = widget.config.query.insightId || undefined;
  const insights = useInsights();

  // The dashboard's live time range and resolved variables (WS-01/WS-02),
  // read from the same stores the real panel render uses. Test query must
  // build the *same* request body as `useWidgetQuery` — otherwise the
  // server-side binder has no range/interval to expand `$__timeGroup` /
  // `$__timeFilter` / `$__interval` against, and no values for `$site` &c.,
  // and the test fails on SQL that renders fine on the canvas.
  const range = useTimeStore((s) => s.range);
  const now = useTimeStore((s) => s.now);
  const resolvedVars = useVariableStore((s) => s.resolved);
  const variables = toQueryVariables(resolvedVars);

  const client = useStarterClient();
  const test = useMutation<QueryResponse, Error>({
    mutationFn: () => {
      const resolved = resolveTimeRange(range, now);
      const req: QueryRequest = {
        sql: sql.trim(),
        time_range: {
          from: resolved.from.toISOString(),
          to: resolved.to.toISOString(),
        },
        interval_secs: intervalSecs(resolved),
        ...(variables.length > 0 ? { variables } : {}),
        // Apply the attached insight so the test matches the rendered panel.
        ...(insightId ? { insight: { insight_id: insightId } } : {}),
      };
      return datasourceId
        ? queryDatasource(client, datasourceId, req)
        : runQuery(client, req);
    },
  });
  const canTest = Boolean(datasourceId) && sql.trim().length > 0;

  return (
    <div className="space-y-4">
      <div className="space-y-1.5">
        <Label htmlFor="ed-title">Title</Label>
        <Input
          id="ed-title"
          value={widget.title}
          onChange={(e) => patch({ title: e.target.value })}
        />
      </div>

      <div className="space-y-1.5">
        <Label>Datasource</Label>
        <DatasourcePicker
          value={datasourceId}
          onChange={(id) =>
            patchConfig({ query: { ...widget.config.query, datasourceId: id ?? "" } })
          }
        />
      </div>

      <div className="space-y-1.5">
        <div className="flex items-center justify-between gap-2">
          <Label htmlFor="ed-sql">SQL</Label>
          <AiSqlAssist
            datasourceId={datasourceId}
            currentSql={sql}
            onApply={(v) =>
              patchConfig({ query: { ...widget.config.query, sql: v } })
            }
          />
        </div>
        <SqlEditor
          id="ed-sql"
          value={sql}
          onChange={(v) => patchConfig({ query: { ...widget.config.query, sql: v } })}
          datasourceId={datasourceId}
          minHeight="10rem"
          ariaLabel="Panel SQL"
        />
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="w-full gap-2"
          disabled={!canTest || test.isPending}
          onClick={() => test.mutate()}
        >
          <PlayCircle className="size-4" />
          {test.isPending ? "Running…" : "Test query"}
        </Button>
        {test.isError ? (
          <p role="alert" className="rounded-md bg-destructive/10 px-2 py-1.5 text-xs text-destructive">
            {test.error instanceof Error ? test.error.message : "Query failed."}
          </p>
        ) : test.isSuccess ? (
          <div className="rounded-md bg-accent/20 px-2 py-1.5 text-xs text-muted-foreground">
            <p className="text-foreground">
              {test.data.stats.row_count} row
              {test.data.stats.row_count === 1 ? "" : "s"} ·{" "}
              {test.data.stats.elapsed_ms} ms
              {test.data.stats.truncated ? " · capped" : ""}
            </p>
            {test.data.columns.length > 0 ? (
              <p className="mt-0.5 truncate font-mono">
                {test.data.columns.map((c) => c.name).join(", ")}
              </p>
            ) : null}
          </div>
        ) : null}
      </div>

      <div className="space-y-1.5">
        <Label htmlFor="ed-insight">Insight (post-query transform)</Label>
        <Select
          value={insightId ?? NO_INSIGHT}
          onValueChange={(v) =>
            patchConfig({
              query: {
                ...widget.config.query,
                insightId: v === NO_INSIGHT ? undefined : v,
              },
            })
          }
        >
          <SelectTrigger id="ed-insight">
            <SelectValue
              placeholder={insights.isPending ? "Loading insights…" : "None"}
            />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={NO_INSIGHT}>None</SelectItem>
            {(insights.data ?? []).map((ins) => (
              <SelectItem key={ins.id} value={ins.id}>
                {ins.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          Applies a saved insight's transform to this panel's query result
          before it's drawn.
        </p>
      </div>
    </div>
  );
}
