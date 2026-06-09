import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Check, Pencil, Plus, Trash2, X } from "lucide-react";
import { StarterError } from "@nube/starter-client-ts";
import { Button } from "@nube/starter-ui-kit/components/button";
import { Input } from "@nube/starter-ui-kit/components/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@nube/starter-ui-kit/components/dialog";

import type { DashboardSummary } from "@/api/types";
import { DashboardFormDialog } from "@/features/dashboards/DashboardFormDialog";
import { useDashboards } from "@/features/dashboards/useDashboards";
import {
  useDeleteDashboard,
  useUpdateDashboard,
} from "@/features/dashboards/useDashboardMutations";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Dashboard management: a table of the tenant's dashboards with open,
// inline rename, and delete over the real endpoints. Loading / empty /
// error states throughout (F0). Rename PATCHes name only — the slug is
// derived server-side on create and left stable here so existing links
// keep working; the slug column is shown read-only for reference.
export function DashboardsPage() {
  const { data, isPending, isError, error } = useDashboards();
  const [creating, setCreating] = useState(false);
  const [toDelete, setToDelete] = useState<DashboardSummary | null>(null);

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold tracking-tight">Dashboards</h2>
        <Button size="sm" className="gap-2" onClick={() => setCreating(true)}>
          <Plus className="size-4" />
          New dashboard
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {isPending ? (
          <Loading label="Loading dashboards…" />
        ) : isError ? (
          <ErrorState
            message={error instanceof Error ? error.message : undefined}
          />
        ) : data.length === 0 ? (
          <Empty
            title="No dashboards yet"
            description="Create your first dashboard to start building panels."
          />
        ) : (
          <div className="glass overflow-hidden rounded-xl">
            <table className="w-full text-sm">
              <thead className="bg-card/60 text-left text-muted-foreground">
                <tr>
                  <th className="px-4 py-2.5 font-medium">Name</th>
                  <th className="px-4 py-2.5 font-medium">Slug</th>
                  <th className="px-4 py-2.5 text-right font-medium">Actions</th>
                </tr>
              </thead>
              <tbody>
                {data.map((d) => (
                  <DashboardRow
                    key={d.id}
                    dashboard={d}
                    onDelete={() => setToDelete(d)}
                  />
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <DashboardFormDialog open={creating} onOpenChange={setCreating} />
      <DeleteDialog
        dashboard={toDelete}
        onClose={() => setToDelete(null)}
      />
    </div>
  );
}

// One table row: name (inline-editable), slug, and open / rename / delete
// actions. Rename swaps the name cell for an input; the row reverts on
// cancel or after a successful save.
function DashboardRow({
  dashboard,
  onDelete,
}: {
  dashboard: DashboardSummary;
  onDelete: () => void;
}) {
  const navigate = useNavigate();
  const update = useUpdateDashboard();
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(dashboard.name);

  function save() {
    const trimmed = name.trim();
    if (!trimmed || trimmed === dashboard.name) {
      setEditing(false);
      setName(dashboard.name);
      return;
    }
    update.mutate(
      { slug: dashboard.slug, patch: { name: trimmed } },
      { onSuccess: () => setEditing(false) },
    );
  }

  return (
    <tr className="border-t border-border/60 hover:bg-accent/20">
      <td className="px-4 py-2.5">
        {editing ? (
          <Input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") save();
              if (e.key === "Escape") {
                setEditing(false);
                setName(dashboard.name);
              }
            }}
            className="h-8"
            aria-label="Dashboard name"
          />
        ) : (
          <button
            type="button"
            className="font-medium text-foreground hover:text-primary"
            onClick={() => navigate(`/d/${dashboard.slug}`)}
          >
            {dashboard.name}
          </button>
        )}
      </td>
      <td className="px-4 py-2.5 font-mono text-xs text-muted-foreground">
        {dashboard.slug}
      </td>
      <td className="px-4 py-2.5">
        <div className="flex items-center justify-end gap-1">
          {editing ? (
            <>
              <IconButton
                label="Save name"
                onClick={save}
                disabled={update.isPending}
              >
                <Check className="size-4" />
              </IconButton>
              <IconButton
                label="Cancel rename"
                onClick={() => {
                  setEditing(false);
                  setName(dashboard.name);
                }}
              >
                <X className="size-4" />
              </IconButton>
            </>
          ) : (
            <>
              <IconButton label="Open" onClick={() => navigate(`/d/${dashboard.slug}`)}>
                Open
              </IconButton>
              <IconButton label="Rename" onClick={() => setEditing(true)}>
                <Pencil className="size-4" />
              </IconButton>
              <IconButton label="Delete" onClick={onDelete} destructive>
                <Trash2 className="size-4" />
              </IconButton>
            </>
          )}
        </div>
        {update.isError ? (
          <p role="alert" className="mt-1 text-right text-xs text-destructive">
            {update.error instanceof StarterError && update.error.status === 409
              ? "That name's slug is already taken."
              : "Couldn't rename."}
          </p>
        ) : null}
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

// Confirm before delete — removing a dashboard drops its panels too.
function DeleteDialog({
  dashboard,
  onClose,
}: {
  dashboard: DashboardSummary | null;
  onClose: () => void;
}) {
  const del = useDeleteDashboard();
  return (
    <Dialog open={dashboard !== null} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="glass max-w-md">
        <DialogHeader>
          <DialogTitle>Delete dashboard</DialogTitle>
          <DialogDescription>
            Delete <span className="text-foreground">{dashboard?.name}</span> and
            all its panels? This can't be undone.
          </DialogDescription>
        </DialogHeader>
        {del.isError ? (
          <p role="alert" className="text-sm text-destructive">
            Couldn't delete the dashboard.
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
              if (!dashboard) return;
              del.mutate(dashboard.slug, { onSuccess: onClose });
            }}
          >
            {del.isPending ? "Deleting…" : "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
