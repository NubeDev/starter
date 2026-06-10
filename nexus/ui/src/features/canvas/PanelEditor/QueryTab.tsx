import { useMutation } from "@tanstack/react-query";
import { useStarterClient } from "@nube/starter-client-react";
import { PlayCircle } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import type { QueryResponse } from "@/api/types";
import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import type { EditorDraft } from "@/features/canvas/PanelEditor/useEditorDraft";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { SqlEditor } from "@/features/sql-editor";

// Query tab: datasource + SQL (the WS-03 CodeMirror editor) + a Test run
// that reports row count / columns / timing without mutating the panel.
// The title also lives here as the first thing a user sets. Edits flow
// into the editor draft; nothing persists until Save.
export function QueryTab({ draft }: { draft: EditorDraft }) {
  const { widget, patch, patchConfig } = draft;
  const datasourceId = widget.config.query.datasourceId || undefined;
  const sql = widget.config.query.sql;

  const client = useStarterClient();
  const test = useMutation<QueryResponse, Error>({
    mutationFn: () => {
      const req = { sql: sql.trim() };
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
        <Label htmlFor="ed-sql">SQL</Label>
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
    </div>
  );
}
