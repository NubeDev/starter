import { LayoutDashboard, Plus } from "lucide-react";
import { NavLink } from "react-router-dom";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarItem,
} from "@nube/starter-ui-kit/components/sidebar";

import { ExtensionSlot } from "@/extensions/ExtensionSlot";

// App navigation on the kit's `Sidebar` primitive (not a hand-rolled
// fixed div — the kit gives collapse + a11y for free). The dashboard
// list will hydrate from `nexus-api` once the client is codegen'd; until
// then the group renders its extension contribution points and the host
// nav. Extensions add nav under `sidebar-nav` and status under
// `sidebar` — generic slots, any remote can fill them (D7).
export function AppSidebar() {
  return (
    <Sidebar className="glass border-r border-border/60">
      <SidebarHeader className="px-3 py-4">
        <div className="flex items-center gap-2">
          <span className="ring-glow grid size-8 place-items-center rounded-lg bg-primary/15 text-primary">
            <LayoutDashboard className="size-4" />
          </span>
          <span className="text-base font-semibold tracking-tight">Nexus</span>
        </div>
      </SidebarHeader>
      <SidebarContent className="scrollbar-thin px-2">
        <SidebarGroup>
          <SidebarGroupLabel>Dashboards</SidebarGroupLabel>
          <SidebarGroupContent>
            <NavLink to="/" end>
              {({ isActive }) => (
                <SidebarItem active={isActive} icon={<LayoutDashboard />}>
                  Overview
                </SidebarItem>
              )}
            </NavLink>
            <SidebarItem icon={<Plus />} className="text-muted-foreground">
              New dashboard
            </SidebarItem>
          </SidebarGroupContent>
        </SidebarGroup>
        <SidebarGroup>
          <SidebarGroupContent>
            <ExtensionSlot id="sidebar-nav" />
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <ExtensionSlot id="sidebar" />
    </Sidebar>
  );
}
