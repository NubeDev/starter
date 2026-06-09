import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Play } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import type { QueryResponse } from "@/api/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { KindPicker } from "@/features/query-editor/KindPicker";
import {
  QueryHistoryDrawer,
  useRefreshQueryHistory,
} from "@/features/query-editor/QueryHistoryDrawer";
import { QuickQueries } from "@/features/query-editor/QuickQueries";
import { ResultGrid } from "@/features/query-editor/ResultGrid";
import { SqlEditor } from "@/features/sql-editor";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Ad-hoc SQL Explorer: pick a datasource, author SQL, run it against
// `POST /query`, and see the real rows. The query fires on demand (a
// mutation), never on keystroke — raw SQL is expensive and server-capped.
// Built on the codegen'd client (D2: warehouse-explorer is endpoint-bound,
// so we own this against the Nexus contract).
// Whether the explorer authors raw SQL or invokes a declarative query-kind
// (WS-10). Kind-mode runs a named, server-validated query — no SQL to type.
type Mode = "sql" | "kind";

export function Explore() {
  const client = useStarterClient();
  const refreshHistory = useRefreshQueryHistory();
  const [datasourceId, setDatasourceId] = useState<string | undefined>();
  const [sql, setSql] = useState("");
  const [mode, setMode] = useState<Mode>("sql");
  const [kind, setKind] = useState<string | undefined>();

  // Takes the SQL to run as the mutation variable rather than reading `sql`
  // from state, so a quick-add chip can run the exact query it just inserted
  // without waiting for a state update to flush. A datasource-scoped run is
  // recorded server-side, so refresh the history drawer once it settles.
  const run = useMutation<QueryResponse, Error, string>({
    mutationFn: (toRun) =>
      datasourceId
        ? queryDatasource(client, datasourceId, { sql: toRun })
        : runQuery(client, { sql: toRun }),
    onSettled: () => refreshHistory(),
  });

  // Kind-mode run: invoke the selected kind by name. The kind's SQL is bound
  // server-side from the registry, so the body carries only the empty `sql`
  // and the `kind` name; this minimal picker sends no params yet (defaulted
  // kinds run as-is; the schema-driven params form is a WS-04 follow-up).
  const runKind = useMutation<QueryResponse, Error, string>({
    mutationFn: (name) => runQuery(client, { sql: "", kind: name }),
    onSettled: () => refreshHistory(),
  });

  // Insert a query into the editor and run it in one go — the discovery path.
  const runSql = (toRun: string) => {
    setSql(toRun);
    run.mutate(toRun);
  };

  const active = mode === "sql" ? run : runKind;
  // SQL-mode needs a datasource + SQL; kind-mode needs a selected kind.
  const canRun =
    mode === "sql"
      ? sql.trim().length > 0 && !!datasourceId && !run.isPending
      : !!kind && !runKind.isPending;
  const onRun = () => {
    if (mode === "sql") {
      run.mutate(sql);
    } else if (kind) {
      runKind.mutate(kind);
    }
  };

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="glass flex flex-col gap-3 rounded-xl p-4">
        <div className="flex items-center gap-3">
          <div className="inline-flex overflow-hidden rounded-md border">
            <button
              type="button"
              className={`px-3 py-1.5 text-sm ${mode === "sql" ? "bg-accent" : ""}`}
              onClick={() => setMode("sql")}
            >
              SQL
            </button>
            <button
              type="button"
              className={`px-3 py-1.5 text-sm ${mode === "kind" ? "bg-accent" : ""}`}
              onClick={() => setMode("kind")}
            >
              Kind
            </button>
          </div>
          {mode === "sql" ? (
            <DatasourcePicker value={datasourceId} onChange={setDatasourceId} />
          ) : (
            <KindPicker value={kind} onChange={setKind} />
          )}
          <Button
            className="ms-auto gap-2"
            disabled={!canRun}
            onClick={onRun}
          >
            <Play className="size-4" />
            {active.isPending ? "Running…" : "Run"}
          </Button>
        </div>
        {mode === "sql" ? (
          <>
            <QuickQueries datasourceId={datasourceId} onRun={runSql} />
            <QueryHistoryDrawer onRecall={setSql} onRerun={runSql} />
            <SqlEditor
              value={sql}
              onChange={setSql}
              datasourceId={datasourceId}
              minHeight="8rem"
              ariaLabel="SQL query"
            />
          </>
        ) : (
          <p className="text-xs text-muted-foreground">
            Runs a declarative query-kind by name. The query and its tenant
            isolation are defined server-side; pick a kind and run.
          </p>
        )}
        {active.data?.stats ? (
          <p className="text-xs text-muted-foreground">
            {active.data.stats.row_count} rows · {active.data.stats.elapsed_ms} ms
            {active.data.stats.truncated ? " · capped" : ""}
          </p>
        ) : null}
      </div>
      <div className="min-h-0 flex-1">
        {active.isPending ? (
          <Loading label="Running query…" />
        ) : active.isError ? (
          <ErrorState message={active.error.message} />
        ) : active.data ? (
          <ResultGrid result={active.data} />
        ) : null}
      </div>
    </div>
  );
}
