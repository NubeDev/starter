import { useState } from "react";
import { Plus } from "lucide-react";

import { SidebarMenuButton } from "@/components/ui/sidebar";
import { DashboardFormDialog } from "@/features/dashboards/DashboardFormDialog";

// Sidebar action that opens the create-dashboard dialog. Kept separate
// from the list so the list component stays a pure read of `/dashboards`.
export function NewDashboardButton() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <SidebarMenuButton
        tooltip="New dashboard"
        className="text-muted-foreground"
        onClick={() => setOpen(true)}
      >
        <Plus />
        <span>New dashboard</span>
      </SidebarMenuButton>
      <DashboardFormDialog open={open} onOpenChange={setOpen} />
    </>
  );
}
