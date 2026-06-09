import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Play } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import type { QueryResponse } from "@/api/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
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
export function Explore() {
  const client = useStarterClient();
  const [datasourceId, setDatasourceId] = useState<string | undefined>();
  const [sql, setSql] = useState("");

  // Takes the SQL to run as the mutation variable rather than reading `sql`
  // from state, so a quick-add chip can run the exact query it just inserted
  // without waiting for a state update to flush.
  const run = useMutation<QueryResponse, Error, string>({
    mutationFn: (toRun) =>
      datasourceId
        ? queryDatasource(client, datasourceId, { sql: toRun })
        : runQuery(client, { sql: toRun }),
  });

  // Insert a query into the editor and run it in one go — the discovery path.
  const runSql = (toRun: string) => {
    setSql(toRun);
    run.mutate(toRun);
  };

  // A datasource must be chosen before running — the query is scoped to it.
  const canRun = sql.trim().length > 0 && !!datasourceId && !run.isPending;

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="glass flex flex-col gap-3 rounded-xl p-4">
        <div className="flex items-center gap-3">
          <DatasourcePicker value={datasourceId} onChange={setDatasourceId} />
          <Button
            className="ms-auto gap-2"
            disabled={!canRun}
            onClick={() => run.mutate(sql)}
          >
            <Play className="size-4" />
            {run.isPending ? "Running…" : "Run"}
          </Button>
        </div>
        <QuickQueries datasourceId={datasourceId} onRun={runSql} />
        <SqlEditor
          value={sql}
          onChange={setSql}
          datasourceId={datasourceId}
          minHeight="8rem"
          ariaLabel="SQL query"
        />
        {run.data?.stats ? (
          <p className="text-xs text-muted-foreground">
            {run.data.stats.row_count} rows · {run.data.stats.elapsed_ms} ms
            {run.data.stats.truncated ? " · capped" : ""}
          </p>
        ) : null}
      </div>
      <div className="min-h-0 flex-1">
        {run.isPending ? (
          <Loading label="Running query…" />
        ) : run.isError ? (
          <ErrorState message={run.error.message} />
        ) : run.data ? (
          <ResultGrid result={run.data} />
        ) : null}
      </div>
    </div>
  );
}
