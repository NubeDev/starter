import { useState } from "react";
import { Braces, Eye, Pencil, Plus, Redo2, Share2, Undo2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { Dashboard } from "@/data/types";
import { useUiStore } from "@/store/ui";
import { useRedo, useUndo } from "@/features/audit/useUndoRedo";
import { AddWidgetDialog } from "@/features/canvas/AddWidgetDialog";
import { ShareDashboardDialog } from "@/features/dashboards/ShareDashboardDialog";
import { TimeRangePicker } from "@/features/time/TimeRangePicker";
import { RefreshControl } from "@/features/time/RefreshControl";
import { VariableEditorDialog } from "@/features/variables/VariableEditorDialog";

// The dashboard's header strip: its name, the global time-range picker +
// refresh control, an Add-panel action (edit mode only), and the view/edit
// toggle. Edit mode lives in the shared UI store (the canvas reads it to
// enable drag/resize), so toggling here unlocks the grid everywhere at once.
// The time picker + refresh feed the shared time store every panel query
// resolves against.
export function DashboardToolbar({ dashboard }: { dashboard: Dashboard }) {
  const editing = useUiStore((s) => s.editMode);
  const toggle = useUiStore((s) => s.toggleEditMode);
  const [adding, setAdding] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [editingVars, setEditingVars] = useState(false);
  // Undo/redo target the caller's most recent change group (per-actor, bodyless)
  // and invalidate the whole nexus query tree on success, so the canvas
  // refreshes with the reverted/re-applied state. The same hooks back the
  // global Cmd/Ctrl+Z shortcut (AppShell); these buttons make the action
  // discoverable while editing.
  const undo = useUndo();
  const redo = useRedo();

  return (
    <div className="flex items-center justify-between gap-3">
      <h2 className="text-balance text-base font-semibold tracking-tight">
        {dashboard.name}
      </h2>
      <div className="flex items-center gap-2">
        <TimeRangePicker />
        <RefreshControl />
        {editing ? (
          <>
            <Button
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => undo.mutate()}
              disabled={undo.isPending}
              title="Undo (Ctrl/Cmd+Z)"
              aria-label="Undo"
            >
              <Undo2 className="size-4" />
              Undo
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => redo.mutate()}
              disabled={redo.isPending}
              title="Redo (Ctrl/Cmd+Shift+Z)"
              aria-label="Redo"
            >
              <Redo2 className="size-4" />
              Redo
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => setAdding(true)}
            >
              <Plus className="size-4" />
              Add panel
            </Button>
            <Button
              variant="outline"
              size="sm"
              className="gap-2"
              onClick={() => setEditingVars(true)}
            >
              <Braces className="size-4" />
              Variables
            </Button>
          </>
        ) : null}
        <Button
          variant="outline"
          size="sm"
          className="gap-2"
          onClick={() => setSharing(true)}
        >
          <Share2 className="size-4" />
          Share
        </Button>
        <Button
          variant={editing ? "default" : "outline"}
          size="sm"
          className="gap-2"
          onClick={toggle}
        >
          {editing ? <Eye className="size-4" /> : <Pencil className="size-4" />}
          {editing ? "Done" : "Edit"}
        </Button>
      </div>
      <AddWidgetDialog
        dashboard={dashboard}
        open={adding}
        onOpenChange={setAdding}
      />
      <ShareDashboardDialog
        dashboardId={dashboard.id}
        open={sharing}
        onOpenChange={setSharing}
      />
      <VariableEditorDialog
        slug={dashboard.slug}
        open={editingVars}
        onOpenChange={setEditingVars}
      />
    </div>
  );
}
