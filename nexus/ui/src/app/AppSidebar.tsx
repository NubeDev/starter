import { LayoutDashboard, LayoutGrid } from "lucide-react";
import { NavLink } from "react-router-dom";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";

import { useLayout } from "@/app/LayoutProvider";
import { SidebarUser } from "@/app/SidebarUser";
import { ExtensionSlot } from "@/extensions/ExtensionSlot";
import { NewDashboardButton } from "@/features/dashboards/NewDashboardButton";
import { NavTree } from "@/features/nav/NavTree";

// App navigation on the canonical shadcn `Sidebar`. Its variant
// (floating/inset/sidebar) and collapse mode are driven by the layout
// provider so the user can reshape the shell at runtime. Dashboard items
// will hydrate from `nexus-api` once the client is codegen'd; extensions
// add nav under `sidebar-nav` (generic slots, D7).
export function AppSidebar() {
  const { variant, collapsible } = useLayout();
  return (
    <Sidebar variant={variant} collapsible={collapsible}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            {/* Brand rendered as a menu button so it inherits the same
                icon-mode centering as the nav items below — otherwise the
                logo sits left-aligned and looks offset when collapsed. */}
            <SidebarMenuButton
              size="lg"
              className="gap-2 hover:bg-transparent active:bg-transparent"
            >
              <span className="ring-glow grid size-8 shrink-0 place-items-center rounded-lg bg-primary/15 text-primary">
                <LayoutDashboard className="size-4" />
              </span>
              <span className="truncate text-base font-semibold tracking-tight">
                Nexus
              </span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        {/* The primary navigation is the access-filtered nav tree (WS-13): one
            place that lists dashboard mounts and static pages, gated per node.
            It replaces the old hardcoded Dashboards/Data/Manage groups — the
            static pages now live as `route` nodes seeded into the tree. */}
        <SidebarGroup>
          <SidebarGroupLabel>Navigation</SidebarGroupLabel>
          {/* Build/organise the tree (add groups, mount pages, manage access). */}
          <SidebarGroupAction asChild title="Edit navigation">
            <NavLink to="/nav" aria-label="Edit navigation">
              <LayoutGrid />
            </NavLink>
          </SidebarGroupAction>
          <NavTree />
          <SidebarMenu>
            <SidebarMenuItem>
              <NavLink to="/dashboards" end>
                {({ isActive }) => (
                  <SidebarMenuButton
                    isActive={isActive}
                    tooltip="Manage dashboards"
                    className="text-muted-foreground"
                  >
                    <LayoutGrid />
                    <span>Manage dashboards</span>
                  </SidebarMenuButton>
                )}
              </NavLink>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <NewDashboardButton />
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroup>
        <SidebarGroup>
          <ExtensionSlot id="sidebar-nav" />
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarUser />
      </SidebarFooter>
      <ExtensionSlot id="sidebar" />
      <SidebarRail />
    </Sidebar>
  );
}
