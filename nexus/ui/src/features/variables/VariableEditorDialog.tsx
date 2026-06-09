import { useState } from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";

import type { CreateVariableRequest } from "@/api/types";
import { useVariableStore } from "@/store/variables";
import { VariableForm } from "@/features/variables/VariableForm";
import {
  useCreateVariable,
  useRemoveVariable,
  useUpdateVariable,
} from "@/features/variables/useVariableMutations";

// Manage a dashboard's variables (item 4): list the defined variables, add
// a new one, edit, or delete. Opened from the toolbar in edit mode. The
// resolved set in the store is the list source (already loaded for the
// bar); CRUD goes through the mutation hooks, which invalidate the variable
// query so the list and bar re-resolve.
type Mode = { view: "list" } | { view: "new" } | { view: "edit"; id: string };

export function VariableEditorDialog({
  slug,
  open,
  onOpenChange,
}: {
  slug: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const resolved = useVariableStore((s) => s.resolved);
  const create = useCreateVariable(slug);
  const update = useUpdateVariable(slug);
  const remove = useRemoveVariable(slug);
  const [mode, setMode] = useState<Mode>({ view: "list" });

  const editing =
    mode.view === "edit" ? resolved.find((v) => v.id === mode.id) : undefined;

  function onCreate(payload: CreateVariableRequest) {
    create.mutate(payload, { onSuccess: () => setMode({ view: "list" }) });
  }
  function onUpdate(id: string, payload: CreateVariableRequest) {
    // The form yields a create-shaped payload; for an update we send the
    // same fields as a (full) patch, preserving the current selection.
    update.mutate(
      { id, patch: { ...payload } },
      { onSuccess: () => setMode({ view: "list" }) },
    );
  }

  return (
    <Dialog open={open} onOpenChange={(o) => { onOpenChange(o); if (!o) setMode({ view: "list" }); }}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Dashboard variables</DialogTitle>
          <DialogDescription>
            Variables let panels re-query against a chosen value (e.g.
            <code> $region</code>). Pick values from the bar above the canvas.
          </DialogDescription>
        </DialogHeader>

        {mode.view === "list" ? (
          <div className="space-y-3">
            {resolved.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No variables yet. Add one to parameterise this dashboard.
              </p>
            ) : (
              <ul className="divide-y rounded-md border">
                {resolved.map((v) => (
                  <li
                    key={v.id}
                    className="flex items-center justify-between gap-3 px-3 py-2"
                  >
                    <div className="min-w-0">
                      <span className="font-mono text-sm">${v.name}</span>
                      <span className="ml-2 text-xs text-muted-foreground">
                        {v.kind}
                        {v.multi ? " · multi" : ""}
                        {v.hidden ? " · hidden" : ""}
                      </span>
                    </div>
                    <div className="flex shrink-0 gap-1">
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`Edit ${v.name}`}
                        onClick={() => setMode({ view: "edit", id: v.id })}
                      >
                        <Pencil className="size-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        aria-label={`Delete ${v.name}`}
                        onClick={() => remove.mutate(v.id)}
                      >
                        <Trash2 className="size-4" />
                      </Button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
            <Button className="gap-2" onClick={() => setMode({ view: "new" })}>
              <Plus className="size-4" />
              Add variable
            </Button>
          </div>
        ) : null}

        {mode.view === "new" ? (
          <VariableForm
            submitLabel="Create"
            onSubmit={onCreate}
            onCancel={() => setMode({ view: "list" })}
          />
        ) : null}

        {mode.view === "edit" && editing ? (
          <VariableForm
            submitLabel="Save"
            initial={{
              id: editing.id,
              dashboard_id: "",
              name: editing.name,
              label: editing.label ?? null,
              kind: editing.kind,
              options_config: editing.optionsConfig,
              current: [...editing.current],
              multi: editing.multi,
              include_all: editing.includeAll,
              hidden: editing.hidden,
              sort_order: editing.sortOrder,
            }}
            onSubmit={(payload) => onUpdate(editing.id, payload)}
            onCancel={() => setMode({ view: "list" })}
          />
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
