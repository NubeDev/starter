import { Star } from "lucide-react";
import { NavLink } from "react-router-dom";
import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";

import { dashboardIcon } from "@/features/dashboards/appearance";
import { useDashboards } from "@/features/dashboards/useDashboards";
import { useStarredDashboards } from "@/features/me/useStarredDashboards";

// The caller's starred dashboards, pinned at the top of the sidebar for quick
// access. Per-user (each caller stars their own), so the list is derived from
// the settings bag intersected with the tenant's dashboards. Renders nothing
// when the user has starred none — it must never occupy space empty.
export function SidebarStarred() {
  const { data } = useDashboards();
  const { starredIds } = useStarredDashboards();

  const starred = (data ?? []).filter((d) => starredIds.has(d.id));
  if (starred.length === 0) return null;

  return (
    <SidebarGroup>
      <SidebarGroupLabel>
        <Star className="me-1.5 size-3.5 fill-amber-400 text-amber-400" />
        Starred
      </SidebarGroupLabel>
      <SidebarMenu>
        {starred.map((d) => {
          const Icon = dashboardIcon(d.icon);
          return (
            <SidebarMenuItem key={d.id}>
              <NavLink to={`/d/${d.slug}`}>
                {({ isActive }) => (
                  <SidebarMenuButton isActive={isActive} tooltip={d.name}>
                    {/* Tint with the dashboard's accent, matching the main
                        dashboard list. */}
                    <Icon style={{ color: `hsl(${d.accent})` }} />
                    <span>{d.name}</span>
                  </SidebarMenuButton>
                )}
              </NavLink>
            </SidebarMenuItem>
          );
        })}
      </SidebarMenu>
    </SidebarGroup>
  );
}
