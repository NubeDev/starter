import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Pencil, Plus, Trash2, Upload } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";
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
import { EditDashboardDialog } from "@/features/dashboards/EditDashboardDialog";
import { dashboardIcon } from "@/features/dashboards/appearance";
import { useDashboards } from "@/features/dashboards/useDashboards";
import { useDeleteDashboard } from "@/features/dashboards/useDashboardMutations";
import { Empty } from "@/features/state/Empty";
import { ErrorState } from "@/features/state/ErrorState";
import { Loading } from "@/features/state/Loading";

// Dashboard management: a table of the tenant's dashboards with open,
// inline rename, and delete over the real endpoints. Loading / empty /
// error states throughout (F0). Edit opens the shared dashboard form
// (name + icon + accent) reused from create; the slug stays stable so
// existing links keep working and is shown read-only for reference.
export function DashboardsPage() {
  const navigate = useNavigate();
  const { data, isPending, isError, error } = useDashboards();
  const [creating, setCreating] = useState(false);
  const [toEdit, setToEdit] = useState<DashboardSummary | null>(null);
  const [toDelete, setToDelete] = useState<DashboardSummary | null>(null);

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between">
        <h2 className="text-base font-semibold tracking-tight">Dashboards</h2>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            className="gap-2"
            onClick={() => navigate("/import")}
          >
            <Upload className="size-4" />
            Import
          </Button>
          <Button size="sm" className="gap-2" onClick={() => setCreating(true)}>
            <Plus className="size-4" />
            New dashboard
          </Button>
        </div>
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
                    onEdit={() => setToEdit(d)}
                    onDelete={() => setToDelete(d)}
                  />
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <DashboardFormDialog open={creating} onOpenChange={setCreating} />
      <EditDashboardDialog
        dashboard={toEdit}
        onClose={() => setToEdit(null)}
      />
      <DeleteDialog dashboard={toDelete} onClose={() => setToDelete(null)} />
    </div>
  );
}

// One table row: an accent-tinted icon badge, the name, the slug, and
// open / edit / delete actions. Edit opens the shared form dialog (name +
// icon + accent); the row itself holds no edit state.
function DashboardRow({
  dashboard,
  onEdit,
  onDelete,
}: {
  dashboard: DashboardSummary;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const navigate = useNavigate();
  const Icon = dashboardIcon(dashboard.icon);

  return (
    <tr className="border-t border-border/60 hover:bg-accent/20">
      <td className="px-4 py-2.5">
        <button
          type="button"
          className="flex items-center gap-2.5 text-left font-medium text-foreground hover:text-primary"
          onClick={() => navigate(`/d/${dashboard.slug}`)}
        >
          <span
            className="grid size-7 shrink-0 place-items-center rounded-lg"
            style={{
              background: `hsl(${dashboard.accent} / 0.15)`,
              color: `hsl(${dashboard.accent})`,
            }}
          >
            <Icon className="size-4" />
          </span>
          {dashboard.name}
        </button>
      </td>
      <td className="px-4 py-2.5 font-mono text-xs text-muted-foreground">
        {dashboard.slug}
      </td>
      <td className="px-4 py-2.5">
        <div className="flex items-center justify-end gap-1">
          <IconButton label="Open" onClick={() => navigate(`/d/${dashboard.slug}`)}>
            Open
          </IconButton>
          <IconButton label="Edit" onClick={onEdit}>
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
