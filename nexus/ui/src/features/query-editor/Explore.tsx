import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Play } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Textarea } from "@nube/starter-ui-kit/components/textarea";

import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import type { QueryResponse } from "@/api/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { ResultGrid } from "@/features/query-editor/ResultGrid";
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

  const run = useMutation<QueryResponse, Error>({
    mutationFn: () =>
      datasourceId
        ? queryDatasource(client, datasourceId, { sql })
        : runQuery(client, { sql }),
  });

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
            onClick={() => run.mutate()}
          >
            <Play className="size-4" />
            {run.isPending ? "Running…" : "Run"}
          </Button>
        </div>
        <Textarea
          value={sql}
          onChange={(e) => setSql(e.target.value)}
          placeholder="select … from … where … limit 100"
          spellCheck={false}
          className="tabular min-h-32 resize-y font-mono text-sm"
          aria-label="SQL query"
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
