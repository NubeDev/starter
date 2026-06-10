import { LayoutDashboard } from "lucide-react";
import { NavLink } from "react-router-dom";
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";

import { dashboardIcon } from "@/features/dashboards/appearance";
import { useDashboards } from "@/features/dashboards/useDashboards";

// The sidebar's live dashboard list from `GET /dashboards`. Renders the
// loading and empty states inline (F0 — no placeholder rows); errors fall
// back to nothing rather than breaking the nav. Each entry routes to the
// dashboard by slug.
export function SidebarDashboards() {
  const { data, isPending, isError } = useDashboards();

  if (isPending) {
    return (
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton disabled className="text-muted-foreground">
            <LayoutDashboard />
            <span>Loading…</span>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    );
  }

  if (isError || !data || data.length === 0) {
    return (
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton disabled className="text-muted-foreground">
            <LayoutDashboard />
            <span>{isError ? "Unavailable" : "No dashboards"}</span>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    );
  }

  return (
    <SidebarMenu>
      {data.map((d) => {
        const Icon = dashboardIcon(d.icon);
        return (
          <SidebarMenuItem key={d.id}>
            <NavLink to={`/d/${d.slug}`}>
              {({ isActive }) => (
                <SidebarMenuButton isActive={isActive} tooltip={d.name}>
                  {/* Tint the per-dashboard icon with its accent so the list
                      reads at a glance; the label keeps the default colour. */}
                  <Icon style={{ color: `hsl(${d.accent})` }} />
                  <span>{d.name}</span>
                </SidebarMenuButton>
              )}
            </NavLink>
          </SidebarMenuItem>
        );
      })}
    </SidebarMenu>
  );
}
