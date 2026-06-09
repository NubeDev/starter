import { Eye, Pencil } from "lucide-react";
import { Button } from "@nube/starter-ui-kit/components/button";

import type { Dashboard } from "@/data/types";
import { useUiStore } from "@/store/ui";

// The dashboard's header strip: its name and the view/edit toggle. Edit
// mode lives in the shared UI store (the canvas reads it to enable
// drag/resize), so toggling here unlocks the grid everywhere at once.
export function DashboardToolbar({ dashboard }: { dashboard: Dashboard }) {
  const editing = useUiStore((s) => s.editMode);
  const toggle = useUiStore((s) => s.toggleEditMode);

  return (
    <div className="flex items-center justify-between gap-3">
      <h2 className="text-balance text-base font-semibold tracking-tight">
        {dashboard.name}
      </h2>
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
  );
}
