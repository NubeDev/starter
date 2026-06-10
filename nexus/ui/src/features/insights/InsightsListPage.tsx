import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { Pencil, Play, Plus, Sparkles, Trash2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";
import { Input } from "@nube/starter-ui-kit/components/input";
import { Label } from "@nube/starter-ui-kit/components/label";

import type { InsightSummary } from "@/api/types";
import { useInsights } from "@/features/insights/useInsights";
import {
  useRemoveInsight,
  useUpdateInsight,
} from "@/features/insights/useInsightMutations";
import { RunInsightDialog } from "@/features/insights/RunInsightDialog";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Insights management: a table of the tenant's saved Rhai transforms with
// open-in-Workbench, inline rename, and delete over the real endpoints.
// Loading / empty / error states throughout (F0). Mirrors the Dashboards
// list pattern (glass table + per-row actions). "New insight" and a row
// click both route to the Workbench (`/insights/workbench[?id=…]`), which
// owns authoring and the live preview.
export function InsightsListPage() {
  const navigate = useNavigate();
  const { data, isPending, isError, error } = useInsights();
  const [toRun, setToRun] = useState<InsightSummary | null>(null);
  const [toEdit, setToEdit] = useState<InsightSummary | null>(null);
  const [toDelete, setToDelete] = useState<InsightSummary | null>(null);

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-base font-semibold tracking-tight">Insights</h2>
          <p className="text-xs text-muted-foreground">
            Reusable transforms over your query results — author once, apply on
            panels and in Explore.
          </p>
        </div>
        <Button
          size="sm"
          className="gap-2"
          onClick={() => navigate("/insights/workbench")}
        >
          <Plus className="size-4" />
          New insight
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {isPending ? (
          <Loading label="Loading insights…" />
        ) : isError ? (
          <ErrorState
            message={error instanceof Error ? error.message : undefined}
          />
        ) : data.length === 0 ? (
          <Empty
            title="No insights yet"
            description="Open the Workbench to author your first transform, then save it to reuse it on panels and in Explore."
            action={
              <Button
                size="sm"
                className="gap-2"
                onClick={() => navigate("/insights/workbench")}
              >
                <Sparkles className="size-4" />
                Open Workbench
              </Button>
            }
          />
        ) : (
          <div className="glass overflow-hidden rounded-xl">
            <table className="w-full text-sm">
              <thead className="bg-card/60 text-left text-muted-foreground">
                <tr>
                  <th className="px-4 py-2.5 font-medium">Name</th>
                  <th className="px-4 py-2.5 font-medium">Transform</th>
                  <th className="px-4 py-2.5 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody>
                {data.map((ins) => (
                  <InsightRow
                    key={ins.id}
                    insight={ins}
                    onRun={() => setToRun(ins)}
                    onEdit={() => setToEdit(ins)}
                    onDelete={() => setToDelete(ins)}
                  />
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <RunInsightDialog insight={toRun} onClose={() => setToRun(null)} />
      <EditInsightDialog insight={toEdit} onClose={() => setToEdit(null)} />
      <DeleteDialog insight={toDelete} onClose={() => setToDelete(null)} />
    </div>
  );
}

// One table row: an accent-tinted icon badge, the name, a one-line script
// preview, and open / edit / delete actions. Opening routes to the Workbench
// in edit mode; the row itself holds no edit state.
function InsightRow({
  insight,
  onRun,
  onEdit,
  onDelete,
}: {
  insight: InsightSummary;
  onRun: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const navigate = useNavigate();
  const open = () => navigate(`/insights/workbench?id=${insight.id}`);

  return (
    <tr className="border-t border-border/60 hover:bg-accent/20">
      <td className="px-4 py-2.5">
        <button
          type="button"
          className="flex items-center gap-2.5 text-left font-medium text-foreground hover:text-primary"
          onClick={open}
        >
          <span className="grid size-7 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary">
            <Sparkles className="size-4" />
          </span>
          {insight.name}
        </button>
      </td>
      <td className="max-w-0 px-4 py-2.5">
        <code className="block truncate font-mono text-xs text-muted-foreground">
          {insight.script.trim() || "—"}
        </code>
      </td>
      <td className="px-4 py-2.5">
        <div className="flex items-center justify-end gap-1">
          <IconButton label="Run" onClick={onRun}>
            <span className="flex items-center gap-1">
              <Play className="size-3.5" />
              Run
            </span>
          </IconButton>
          <IconButton label="Open" onClick={open}>
            Open
          </IconButton>
          <IconButton label="Rename" onClick={onEdit}>
            <Pencil className="size-4" />
          </IconButton>
          <IconButton label="Delete" onClick={onDelete} destructive>
            <Trash2 className="size-4" />
          </IconButton>
        </div>
      </td>
    </tr>
  );
}

function IconButton({
  label,
  onClick,
  disabled,
  destructive,
  children,
}: {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  destructive?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      disabled={disabled}
      className={`rounded-md px-2 py-1 text-xs text-muted-foreground transition-colors disabled:opacity-50 ${
        destructive
          ? "hover:bg-destructive/15 hover:text-destructive"
          : "hover:bg-accent/40 hover:text-foreground"
      }`}
    >
      {children}
    </button>
  );
}

// Inline rename — a quick edit of the name without leaving the list. Editing
// the script itself happens in the Workbench (Open), where it can be
// compile-checked against a live sample; the rename here only PATCHes `name`.
function EditInsightDialog({
  insight,
  onClose,
}: {
  insight: InsightSummary | null;
  onClose: () => void;
}) {
  const update = useUpdateInsight();
  const [name, setName] = useState("");

  // Seed the field when a new insight is selected; keep the dialog controlled.
  const selectedId = insight?.id ?? null;
  const [seededFor, setSeededFor] = useState<string | null>(null);
  if (insight && seededFor !== selectedId) {
    setSeededFor(selectedId);
    setName(insight.name);
    update.reset();
  }

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!insight) return;
    update.mutate(
      { id: insight.id, body: { name: name.trim() } },
      { onSuccess: onClose },
    );
  }

  return (
    <Dialog open={insight !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="glass max-w-md">
        <DialogHeader>
          <DialogTitle>Rename insight</DialogTitle>
          <DialogDescription>
            Edit the name. To change the transform script, open it in the
            Workbench.
          </DialogDescription>
        </DialogHeader>
        <form className="space-y-3" onSubmit={onSubmit}>
          <div className="space-y-1.5">
            <Label htmlFor="rename-insight">Name</Label>
            <Input
              id="rename-insight"
              value={name}
              onChange={(e) => {
                update.reset();
                setName(e.target.value);
              }}
              autoComplete="off"
              required
            />
          </div>
          {update.isError ? (
            <p role="alert" className="text-sm text-destructive">
              {update.error instanceof Error
                ? update.error.message
                : "Couldn't rename the insight."}
            </p>
          ) : null}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={onClose}
              disabled={update.isPending}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={update.isPending || !name.trim()}>
              {update.isPending ? "Saving…" : "Save"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// Confirm before delete — panels and Explore queries that reference this
// insight will stop applying it.
function DeleteDialog({
  insight,
  onClose,
}: {
  insight: InsightSummary | null;
  onClose: () => void;
}) {
  const del = useRemoveInsight();
  return (
    <Dialog open={insight !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="glass max-w-md">
        <DialogHeader>
          <DialogTitle>Delete insight</DialogTitle>
          <DialogDescription>
            Delete <span className="text-foreground">{insight?.name}</span>?
            Anything referencing it will stop applying the transform. This can't
            be undone.
          </DialogDescription>
        </DialogHeader>
        {del.isError ? (
          <p role="alert" className="text-sm text-destructive">
            Couldn't delete the insight.
          </p>
        ) : null}
        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={del.isPending}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            disabled={del.isPending}
            onClick={() => {
              if (!insight) return;
              del.mutate(insight.id, { onSuccess: onClose });
            }}
          >
            {del.isPending ? "Deleting…" : "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
