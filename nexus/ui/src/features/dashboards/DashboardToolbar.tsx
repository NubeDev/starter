import { useState } from "react";
import { Eye, Pencil, Plus, Share2 } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { Dashboard } from "@/data/types";
import { useUiStore } from "@/store/ui";
import { AddWidgetDialog } from "@/features/canvas/AddWidgetDialog";
import { ShareDashboardDialog } from "@/features/dashboards/ShareDashboardDialog";
import { TimeRangePicker } from "@/features/time/TimeRangePicker";
import { RefreshControl } from "@/features/time/RefreshControl";

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

  return (
    <div className="flex items-center justify-between gap-3">
      <h2 className="text-balance text-base font-semibold tracking-tight">
        {dashboard.name}
      </h2>
      <div className="flex items-center gap-2">
        <TimeRangePicker />
        <RefreshControl />
        {editing ? (
          <Button
            variant="outline"
            size="sm"
            className="gap-2"
            onClick={() => setAdding(true)}
          >
            <Plus className="size-4" />
            Add panel
          </Button>
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
    </div>
  );
}
