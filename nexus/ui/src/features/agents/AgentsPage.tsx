import { useState } from "react";
import { Bot, Pencil, Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import { cn } from "@nube/starter-ui-kit/lib/utils";

import type { AgentSummary } from "@/api/types";
import { useAgent, useAgents, useDeleteAgent } from "@/features/agents/useAgents";
import { AgentChat } from "@/features/agents/AgentChat";
import { AgentFormDialog } from "@/features/agents/AgentFormDialog";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Agents management: a two-pane surface. Left lists the tenant's agents with
// create / edit / delete actions; right hosts the chatbot for the selected
// agent (the test surface) or an empty state. Loading / empty / error states
// throughout (F0). Editing fetches the full agent so the system prompt is
// available in the form.
export function AgentsPage() {
  const { data, isPending, isError, error } = useAgents();
  const remove = useDeleteAgent();
  const [adding, setAdding] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const editing = useAgent(editingId ?? undefined);
  const selected = data?.find((a) => a.id === selectedId) ?? null;

  function onDelete(agent: AgentSummary) {
    if (!window.confirm(`Delete "${agent.name}"? This can't be undone.`)) return;
    remove.mutate(agent.id, {
      onSuccess: () => {
        if (selectedId === agent.id) setSelectedId(null);
      },
    });
  }

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold tracking-tight">Agents</h2>
        <Button size="sm" className="gap-2" onClick={() => setAdding(true)}>
          <Plus className="size-4" />
          New agent
        </Button>
      </div>

      <div className="min-h-0 flex-1">
        {isPending ? (
          <Loading label="Loading agents…" />
        ) : isError ? (
          <ErrorState
            message={error instanceof Error ? error.message : undefined}
          />
        ) : data.length === 0 ? (
          <Empty
            title="No agents"
            description="Define an AI agent to start chatting."
          />
        ) : (
          <div className="grid h-full min-h-0 grid-cols-1 gap-4 lg:grid-cols-[20rem_1fr]">
            <ul className="flex min-h-0 flex-col gap-2 overflow-y-auto scrollbar-thin">
              {data.map((agent) => (
                <AgentRow
                  key={agent.id}
                  agent={agent}
                  selected={agent.id === selectedId}
                  onSelect={() => setSelectedId(agent.id)}
                  onEdit={() => setEditingId(agent.id)}
                  onDelete={() => onDelete(agent)}
                  removing={remove.isPending}
                />
              ))}
            </ul>

            <div className="glass min-h-0 rounded-lg p-4">
              {selected ? (
                <AgentChat agent={selected} />
              ) : (
                <Empty
                  title="Select an agent"
                  description="Pick an agent on the left to chat and check it works."
                />
              )}
            </div>
          </div>
        )}
      </div>

      <AgentFormDialog open={adding} onOpenChange={setAdding} />
      <AgentFormDialog
        open={editingId != null && editing.data != null}
        onOpenChange={(open) => {
          if (!open) setEditingId(null);
        }}
        agent={editing.data}
      />
    </div>
  );
}

function AgentRow({
  agent,
  selected,
  onSelect,
  onEdit,
  onDelete,
  removing,
}: {
  agent: AgentSummary;
  selected: boolean;
  onSelect: () => void;
  onEdit: () => void;
  onDelete: () => void;
  removing: boolean;
}) {
  return (
    <li
      className={cn(
        "glass flex items-center gap-3 rounded-lg px-4 py-3",
        selected && "ring-1 ring-primary/50",
      )}
    >
      <button
        type="button"
        onClick={onSelect}
        className="flex min-w-0 flex-1 items-center gap-3 text-left"
      >
        <span className="grid size-9 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary">
          <Bot className="size-4" />
        </span>
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium text-foreground">
            {agent.name}
          </p>
          <p className="truncate text-xs text-muted-foreground">
            {agent.backend} · {agent.model}
          </p>
        </div>
      </button>

      <Button
        variant="ghost"
        size="icon"
        aria-label={`Edit ${agent.name}`}
        onClick={onEdit}
        className="text-muted-foreground"
      >
        <Pencil className="size-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        aria-label={`Delete ${agent.name}`}
        disabled={removing}
        onClick={onDelete}
        className="text-muted-foreground hover:text-destructive"
      >
        <Trash2 className="size-4" />
      </Button>
    </li>
  );
}
