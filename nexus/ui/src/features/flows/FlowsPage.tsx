import { useState } from "react";
import { AlertTriangle, Pause, Play, Plus, Trash2, Workflow } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { FlowSummary } from "@/api/types";
import { useFlowActions, useFlows } from "@/features/flows/useFlows";
import { FlowBuilder } from "@/features/flows/builder/FlowBuilder";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Flow management: list the tenant's saved ingestion flows with their
// running state and start/stop + delete actions, plus a config editor to
// author new ones — all over the real endpoints. Loading/empty/error
// throughout (F0).
export function FlowsPage() {
  const { data, isPending, isError, error } = useFlows();
  const actions = useFlowActions();
  const [creating, setCreating] = useState(false);

  const busy =
    actions.start.isPending || actions.stop.isPending || actions.remove.isPending;

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold tracking-tight">Flows</h2>
        <Button size="sm" className="gap-2" onClick={() => setCreating(true)}>
          <Plus className="size-4" />
          New flow
        </Button>
      </div>

      <div className="min-h-0 flex-1">
        {isPending ? (
          <Loading label="Loading flows…" />
        ) : isError ? (
          <ErrorState message={error instanceof Error ? error.message : undefined} />
        ) : data.length === 0 ? (
          <Empty
            title="No flows"
            description="Flows are long-running ingestion pipelines. Create one to begin."
          />
        ) : (
          <ul className="flex flex-col gap-2">
            {data.map((flow) => (
              <FlowRow key={flow.id} flow={flow} busy={busy} actions={actions} />
            ))}
          </ul>
        )}
      </div>

      <FlowBuilder open={creating} onOpenChange={setCreating} />
    </div>
  );
}

function FlowRow({
  flow,
  busy,
  actions,
}: {
  flow: FlowSummary;
  busy: boolean;
  actions: ReturnType<typeof useFlowActions>;
}) {
  return (
    <li className="glass flex items-center gap-3 rounded-lg px-4 py-3">
      <span className="grid size-9 place-items-center rounded-lg bg-primary/15 text-primary">
        <Workflow className="size-4" />
      </span>
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-foreground">{flow.name}</p>
        <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span
            className="size-1.5 rounded-full"
            style={{
              backgroundColor: flow.running
                ? "var(--chart-1)"
                : "var(--muted-foreground)",
              boxShadow: flow.running ? "0 0 8px var(--chart-1)" : undefined,
            }}
            aria-hidden
          />
          {flow.running ? "Running" : flow.enabled ? "Stopped" : "Disabled"}
          {flow.metrics.last_started_at && !flow.metrics.last_error ? (
            <span className="text-muted-foreground/70">
              · since {flow.metrics.last_started_at}
            </span>
          ) : null}
        </p>
        {flow.metrics.last_error ? (
          <p
            className="mt-0.5 flex items-center gap-1 truncate text-[11px] text-destructive"
            title={flow.metrics.last_error}
          >
            <AlertTriangle className="size-3 shrink-0" aria-hidden />
            <span className="truncate">{flow.metrics.last_error}</span>
          </p>
        ) : null}
      </div>
      <Button
        variant="outline"
        size="sm"
        className="gap-2"
        disabled={busy}
        onClick={() =>
          flow.running
            ? actions.stop.mutate(flow.id)
            : actions.start.mutate(flow.id)
        }
      >
        {flow.running ? (
          <>
            <Pause className="size-4" /> Stop
          </>
        ) : (
          <>
            <Play className="size-4" /> Start
          </>
        )}
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Delete ${flow.name}`}
        disabled={busy}
        onClick={() => actions.remove.mutate(flow.id)}
        className="text-muted-foreground hover:text-destructive"
      >
        <Trash2 className="size-4" />
      </Button>
    </li>
  );
}
