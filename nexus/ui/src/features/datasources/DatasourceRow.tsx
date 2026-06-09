import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Check, Database, Plug, Trash2, X } from "lucide-react";
import { useStarterClient } from "@nube/starter-client-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import { testDatasource } from "@/api/datasources/test";
import type { DatasourceSummary, TestDatasourceResponse } from "@/api/types";

// One datasource row: name/kind, a Test-connection action that probes the
// stored credentials and shows the result inline, and delete. The test
// result lives in the row (not a shared store) since it's a per-row,
// transient probe.
export function DatasourceRow({
  datasource,
  onRemove,
  removing,
}: {
  datasource: DatasourceSummary;
  onRemove: () => void;
  removing: boolean;
}) {
  const client = useStarterClient();
  const [result, setResult] = useState<TestDatasourceResponse | null>(null);

  const test = useMutation<TestDatasourceResponse, Error>({
    mutationFn: () => testDatasource(client, datasource.id),
    onSuccess: setResult,
    onError: () => setResult({ ok: false, message: "Probe failed", latency_ms: null }),
  });

  return (
    <li className="glass flex items-center gap-3 rounded-lg px-4 py-3">
      <span className="grid size-9 place-items-center rounded-lg bg-primary/15 text-primary">
        <Database className="size-4" />
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-foreground">
          {datasource.name}
        </p>
        <p className="text-xs text-muted-foreground">{datasource.kind}</p>
      </div>

      {result ? <TestResult result={result} /> : null}

      <Button
        variant="outline"
        size="sm"
        className="gap-2"
        disabled={test.isPending}
        onClick={() => test.mutate()}
      >
        <Plug className="size-4" />
        {test.isPending ? "Testing…" : "Test"}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Delete ${datasource.name}`}
        disabled={removing}
        onClick={onRemove}
        className="text-muted-foreground hover:text-destructive"
      >
        <Trash2 className="size-4" />
      </Button>
    </li>
  );
}

function TestResult({ result }: { result: TestDatasourceResponse }) {
  const color = result.ok ? "var(--chart-1)" : "var(--destructive)";
  return (
    <span
      className="flex items-center gap-1.5 text-xs"
      style={{ color }}
      role="status"
    >
      {result.ok ? <Check className="size-3.5" /> : <X className="size-3.5" />}
      {result.ok
        ? result.latency_ms != null
          ? `${result.latency_ms} ms`
          : "OK"
        : (result.message ?? "Failed")}
    </span>
  );
}
