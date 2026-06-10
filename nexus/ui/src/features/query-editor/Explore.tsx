import { useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Layers, Play, Save } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { AiSqlAssist } from "@/features/ai/AiSqlAssist";
import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import type {
  FederatedSourceRef,
  InsightRef,
  QueryRequest,
  QueryResponse,
} from "@/api/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { FederationPicker } from "@/features/query-editor/FederationPicker";
import {
  InsightPicker,
  type InsightMode,
} from "@/features/query-editor/InsightPicker";
import { KindPicker } from "@/features/query-editor/KindPicker";
import {
  QueryHistoryDrawer,
  useRefreshQueryHistory,
} from "@/features/query-editor/QueryHistoryDrawer";
import { QuickQueries } from "@/features/query-editor/QuickQueries";
import { ResultGrid } from "@/features/query-editor/ResultGrid";
import { SaveKindDialog } from "@/features/query-editor/SaveKindDialog";
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
  const [saveOpen, setSaveOpen] = useState(false);

  // Federation (RW-05): optional extra datasources the SQL can JOIN across,
  // each bound to an alias. Folded away by default; when non-empty the run
  // must go through the unscoped `POST /query` since it spans datasources.
  const [showFederation, setShowFederation] = useState(false);
  const [sources, setSources] = useState<FederatedSourceRef[]>([]);

  // Insight (post-query Rhai transform). Authored here so both SQL and kind
  // runs can attach it; params are JSON text parsed on run.
  const [insightMode, setInsightMode] = useState<InsightMode>("none");
  const [insightId, setInsightId] = useState<string | undefined>();
  const [insightScript, setInsightScript] = useState("");
  const [insightParams, setInsightParams] = useState("");
  const [paramsError, setParamsError] = useState<string | null>(null);

  // Only fully-specified rows are sent: an alias and a datasource id are
  // both required for the server to resolve the join. Empty `table` is
  // dropped so a file datasource isn't handed a blank string.
  const cleanSources = useMemo<FederatedSourceRef[]>(
    () =>
      sources
        .filter((s) => s.alias.trim() && s.datasource)
        .map((s) => ({
          alias: s.alias.trim(),
          datasource: s.datasource,
          ...(s.table && s.table.trim() ? { table: s.table.trim() } : {}),
        })),
    [sources],
  );
  const hasFederation = cleanSources.length > 0;

  // Build the `insight` field for a request body, or undefined when off.
  // Precedence mirrors the backend: a stored id wins, else the inline
  // script. Params (if any) are parsed from the JSON textarea; a parse
  // error is surfaced and gates the run (see `onRun`).
  const buildInsight = (): InsightRef | undefined => {
    if (insightMode === "none") return undefined;
    const ref: InsightRef = {};
    if (insightMode === "stored") {
      if (!insightId) return undefined;
      ref.insight_id = insightId;
    } else if (insightMode === "script") {
      if (!insightScript.trim()) return undefined;
      ref.script = insightScript;
    }
    if (insightParams.trim()) {
      ref.params = JSON.parse(insightParams) as unknown;
    }
    return ref;
  };

  // Assemble the additive fields (federation + insight) onto a base body.
  // Throws on bad params JSON — callers run this inside the run handler so
  // the error can be caught and shown inline.
  const withExtras = (base: QueryRequest): QueryRequest => {
    const insight = buildInsight();
    return {
      ...base,
      ...(hasFederation ? { sources: cleanSources } : {}),
      ...(insight ? { insight } : {}),
    };
  };

  // Takes the SQL to run as the mutation variable rather than reading `sql`
  // from state, so a quick-add chip can run the exact query it just inserted
  // without waiting for a state update to flush. A datasource-scoped run is
  // recorded server-side, so refresh the history drawer once it settles.
  // Federation spans datasources, so when present we drop the per-datasource
  // endpoint and run the unscoped `POST /query`.
  const run = useMutation<QueryResponse, Error, string>({
    mutationFn: (toRun) =>
      datasourceId && !hasFederation
        ? queryDatasource(client, datasourceId, withExtras({ sql: toRun }))
        : runQuery(client, withExtras({ sql: toRun })),
    onSettled: () => refreshHistory(),
  });

  // Kind-mode run: invoke the selected kind by name. The kind's SQL is bound
  // server-side from the registry, so the body carries only the empty `sql`
  // and the `kind` name; this minimal picker sends no params yet (defaulted
  // kinds run as-is; the schema-driven params form is a WS-04 follow-up).
  const runKind = useMutation<QueryResponse, Error, string>({
    mutationFn: (name) =>
      runQuery(client, withExtras({ sql: "", kind: name })),
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
    // Validate the params JSON up front so a parse error gates the run and
    // is shown inline rather than throwing inside the mutation.
    if (insightMode !== "none" && insightParams.trim()) {
      try {
        JSON.parse(insightParams);
      } catch (err) {
        setParamsError(
          `Invalid params JSON: ${err instanceof Error ? err.message : String(err)}`,
        );
        return;
      }
    }
    setParamsError(null);
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
          {mode === "sql" ? (
            <div className="ms-auto flex items-center gap-2">
              <Button
                type="button"
                variant={showFederation || hasFederation ? "secondary" : "outline"}
                size="sm"
                className="gap-2"
                onClick={() => setShowFederation((v) => !v)}
              >
                <Layers className="size-4" />
                Federation
                {hasFederation ? (
                  <span className="rounded-full bg-accent px-1.5 text-xs">
                    {cleanSources.length}
                  </span>
                ) : null}
              </Button>
              <AiSqlAssist
                datasourceId={datasourceId}
                currentSql={sql}
                onApply={setSql}
              />
              {sql.trim().length > 0 ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="gap-2"
                  onClick={() => setSaveOpen(true)}
                >
                  <Save className="size-4" />
                  Save as kind
                </Button>
              ) : null}
            </div>
          ) : null}
          <Button
            className={mode === "sql" ? "gap-2" : "ms-auto gap-2"}
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
            {showFederation || hasFederation ? (
              <div className="flex flex-col gap-2 rounded-md border border-dashed p-3">
                <p className="text-xs text-muted-foreground">
                  Add datasources to JOIN across. Each alias becomes a table
                  name in your SQL (referenced as{" "}
                  <code className="font-mono">ds_&lt;alias&gt;</code>). Leave
                  empty for a normal single-datasource query.
                </p>
                <FederationPicker sources={sources} onChange={setSources} />
              </div>
            ) : null}
          </>
        ) : (
          <p className="text-xs text-muted-foreground">
            Runs a declarative query-kind by name. The query and its tenant
            isolation are defined server-side; pick a kind and run.
          </p>
        )}
        <InsightPicker
          mode={insightMode}
          onModeChange={(m) => {
            setInsightMode(m);
            setParamsError(null);
          }}
          insightId={insightId}
          onInsightIdChange={setInsightId}
          script={insightScript}
          onScriptChange={setInsightScript}
          params={insightParams}
          onParamsChange={(p) => {
            setInsightParams(p);
            setParamsError(null);
          }}
          paramsError={paramsError}
        />
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
      <SaveKindDialog
        sql={sql}
        open={saveOpen}
        onClose={() => setSaveOpen(false)}
      />
    </div>
  );
}
