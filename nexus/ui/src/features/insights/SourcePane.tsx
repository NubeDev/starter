import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Database, Play, Sparkles } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@nube/starter-ui-kit/components/select";

import { queryDatasource } from "@/api/datasources/query";
import { runQuery } from "@/api/query/run";
import { getInsight } from "@/api/insights/get";
import type { QueryResponse } from "@/api/types";
import { DatasourcePicker } from "@/features/query-editor/DatasourcePicker";
import { ResultGrid } from "@/features/query-editor/ResultGrid";
import { SqlEditor } from "@/features/sql-editor";
import { useInsights } from "@/features/insights/useInsights";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Cap the sample we feed to preview: rows-in / rows-out is cheap, but a giant
// result would bloat every debounced request. 500 rows is plenty to author and
// see a transform's shape.
const SAMPLE_CAP = 500;

// Starter transforms — clicking fills the editor so the page is never blank.
const TEMPLATES: { label: string; script: string }[] = [
  { label: "Z-score outliers", script: 'zscore("value")' },
  { label: "Rolling average", script: 'rolling_mean("value", 5)' },
  { label: "Detect anomalies", script: 'anomalies("value", 3.0)' },
  {
    label: "Resample hourly",
    script: 'resample("time", "1h", ["value:mean"])',
  },
];

export function SourcePane({
  sample,
  onSample,
  onLoadScript,
  onLoadTemplate,
}: {
  sample: QueryResponse | null;
  onSample: (rows: QueryResponse | null) => void;
  /** Load a saved insight's script into the transform editor. */
  onLoadScript: (script: string) => void;
  /** Fill the transform editor from a starter template. */
  onLoadTemplate: (script: string) => void;
}) {
  const client = useStarterClient();
  const insights = useInsights();
  const [datasourceId, setDatasourceId] = useState<string | undefined>();
  const [sql, setSql] = useState("");

  // Run the query, then cap and store the result as the live preview sample.
  const run = useMutation<QueryResponse, Error, void>({
    mutationFn: async () => {
      const res = datasourceId
        ? await queryDatasource(client, datasourceId, { sql })
        : await runQuery(client, { sql });
      return res.rows.length > SAMPLE_CAP
        ? { ...res, rows: res.rows.slice(0, SAMPLE_CAP) }
        : res;
    },
    onSuccess: (res) => onSample(res),
  });

  // Load a saved insight: fetch its detail and push the script up.
  const loadInsight = useMutation<void, Error, string>({
    mutationFn: async (id) => {
      const detail = await getInsight(client, id);
      onLoadScript(detail.script);
    },
  });

  const canRun = sql.trim().length > 0 && !!datasourceId && !run.isPending;

  return (
    <div className="glass flex h-full min-h-0 flex-col gap-3 rounded-xl p-3">
      <div className="flex items-center gap-2">
        <Database className="size-4 text-muted-foreground" />
        <h3 className="text-sm font-semibold">Source</h3>
        <span className="ms-auto text-xs text-muted-foreground">
          {sample
            ? `${sample.rows.length} sample row${sample.rows.length === 1 ? "" : "s"}`
            : "no sample"}
        </span>
      </div>

      <DatasourcePicker value={datasourceId} onChange={setDatasourceId} />

      <SqlEditor
        value={sql}
        onChange={setSql}
        datasourceId={datasourceId}
        minHeight="7rem"
        ariaLabel="Sample query SQL"
      />

      <Button className="gap-2" disabled={!canRun} onClick={() => run.mutate()}>
        <Play className="size-4" />
        {run.isPending ? "Loading…" : "Run query"}
      </Button>

      <div className="flex flex-col gap-2 rounded-md border border-dashed border-border/60 p-2">
        <div className="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
          <Sparkles className="size-3.5" />
          Quick start
        </div>
        <div className="flex flex-wrap gap-1.5">
          {TEMPLATES.map((tpl) => (
            <button
              key={tpl.label}
              type="button"
              onClick={() => onLoadTemplate(tpl.script)}
              className="rounded-full border border-border/60 px-2.5 py-1 text-xs hover:bg-accent/40"
            >
              {tpl.label}
            </button>
          ))}
        </div>
        <Select
          value=""
          onValueChange={(id) => loadInsight.mutate(id)}
          disabled={insights.isPending || (insights.data?.length ?? 0) === 0}
        >
          <SelectTrigger className="w-full">
            <SelectValue
              placeholder={
                insights.isPending
                  ? "Loading saved insights…"
                  : (insights.data?.length ?? 0) === 0
                    ? "No saved insights yet"
                    : "Load saved insight…"
              }
            />
          </SelectTrigger>
          <SelectContent>
            {(insights.data ?? []).map((ins) => (
              <SelectItem key={ins.id} value={ins.id}>
                {ins.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        {loadInsight.isError ? (
          <p role="alert" className="text-xs text-destructive">
            Couldn't load that insight.
          </p>
        ) : null}
      </div>

      <div className="min-h-0 flex-1">
        {run.isPending ? (
          <Loading label="Running query…" />
        ) : run.isError ? (
          <ErrorState message={run.error.message} />
        ) : sample ? (
          <div className="flex h-full flex-col gap-1.5">
            <span className="text-xs font-medium text-muted-foreground">
              Input sample
            </span>
            <div className="min-h-0 flex-1">
              <ResultGrid result={sample} />
            </div>
          </div>
        ) : (
          <p className="px-1 pt-2 text-xs text-muted-foreground">
            Pick a datasource, write a query, and run it to load a sample. Then
            author a transform in the middle pane and watch the result update
            live.
          </p>
        )}
      </div>
    </div>
  );
}
