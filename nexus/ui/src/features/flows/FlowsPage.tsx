import { useState } from "react";
import {
  AlertTriangle,
  Pause,
  Pencil,
  Play,
  Plus,
  Trash2,
  Workflow,
} from "lucide-react";
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
  // The flow currently open for editing, or null when authoring a new one.
  const [editingId, setEditingId] = useState<string | null>(null);

  // Which flow id (if any) each mutation is currently acting on, so only the
  // affected row shows the busy/disabled state instead of the whole list.
  const pendingId =
    (actions.start.isPending && actions.start.variables) ||
    (actions.stop.isPending && actions.stop.variables) ||
    (actions.remove.isPending && actions.remove.variables) ||
    null;

  // The id + message of the most recent failed start/stop, so the row that the
  // user clicked shows *why* nothing happened (a 400 from an invalid config is
  // otherwise silent).
  const actionError =
    actions.start.error && actions.start.variables
      ? { id: actions.start.variables, message: actions.start.error.message }
      : actions.stop.error && actions.stop.variables
        ? { id: actions.stop.variables, message: actions.stop.error.message }
        : null;

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
              <FlowRow
                key={flow.id}
                flow={flow}
                busy={pendingId === flow.id}
                actionError={
                  actionError?.id === flow.id ? actionError.message : null
                }
                actions={actions}
                onEdit={() => setEditingId(flow.id)}
              />
            ))}
          </ul>
        )}
      </div>

      <FlowBuilder
        open={creating || editingId !== null}
        flowId={editingId}
        onOpenChange={(open) => {
          if (!open) {
            setCreating(false);
            setEditingId(null);
          }
        }}
      />
    </div>
  );
}

function FlowRow({
  flow,
  busy,
  actionError,
  actions,
  onEdit,
}: {
  flow: FlowSummary;
  busy: boolean;
  // A start/stop error for *this* flow (e.g. a 400 from invalid config), shown
  // inline so a click that the server rejected isn't silent.
  actionError: string | null;
  actions: ReturnType<typeof useFlowActions>;
  onEdit: () => void;
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
        {actionError ? (
          <p
            role="alert"
            className="mt-0.5 flex items-center gap-1 truncate text-[11px] text-destructive"
            title={actionError}
          >
            <AlertTriangle className="size-3 shrink-0" aria-hidden />
            <span className="truncate">{actionError}</span>
          </p>
        ) : flow.metrics.last_error ? (
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
        aria-label={`Edit ${flow.name}`}
        // Editing a running flow would let the canvas drift from the live run;
        // stop it first.
        disabled={busy || flow.running}
        title={flow.running ? "Stop the flow to edit it" : undefined}
        onClick={onEdit}
        className="text-muted-foreground hover:text-foreground"
      >
        <Pencil className="size-4" />
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
