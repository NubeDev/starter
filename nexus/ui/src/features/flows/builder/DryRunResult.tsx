import { AlertTriangle } from "lucide-react";

import type { DryRunResponse } from "@/api/types";
import { Empty } from "@/features/state/Empty";

// The dry-run preview: a bounded sample of what the flow's pipeline would emit,
// or the build/runtime error that stopped it. The backend returns the error
// inline (a 200 with `error` set) rather than failing the request, so an
// authoring mistake reads as a message under the editor, not a thrown request.
const NUMERIC = new Set(["int", "float"]);

export function DryRunResult({ result }: { result: DryRunResponse }) {
  if (result.error) {
    return (
      <div
        role="alert"
        className="flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
      >
        <AlertTriangle className="mt-0.5 size-4 shrink-0" />
        <div className="min-w-0">
          <p className="font-medium">The flow didn't build.</p>
          <p className="break-words text-xs opacity-90">{result.error}</p>
        </div>
      </div>
    );
  }

  if (result.rows.length === 0) {
    return (
      <Empty
        title="No rows"
        description="The flow built cleanly but emitted no sample rows."
      />
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-2">
      <p className="text-xs text-muted-foreground">
        {result.stats.row_count} row{result.stats.row_count === 1 ? "" : "s"} in{" "}
        {result.stats.elapsed_ms} ms
        {result.stats.truncated ? " · sample capped" : ""}
      </p>
      <div className="scrollbar-thin min-h-0 flex-1 overflow-auto rounded-lg border border-border/60">
        <table className="w-full text-sm">
          <thead className="sticky top-0 bg-card/90 backdrop-blur">
            <tr className="text-left">
              {result.columns.map((c) => (
                <th key={c.name} className="px-3 py-2 font-medium">
                  <span className="text-foreground">{c.name}</span>
                  <span className="ms-2 text-xs text-muted-foreground">
                    {c.type}
                  </span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {result.rows.map((row, i) => (
              <tr key={i} className="border-t border-border/50">
                {result.columns.map((c) => {
                  const v = (row as Record<string, unknown>)[c.name];
                  return (
                    <td
                      key={c.name}
                      className={`px-3 py-1.5 ${NUMERIC.has(c.type) || c.type === "timestamp" ? "tabular" : "text-foreground"}`}
                    >
                      {v == null ? "—" : String(v)}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
