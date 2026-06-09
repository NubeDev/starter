import { LayoutDashboard, Plus } from "lucide-react";
import { NavLink } from "react-router-dom";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";

import { useLayout } from "@/app/LayoutProvider";
import { ExtensionSlot } from "@/extensions/ExtensionSlot";

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
        <SidebarGroup>
          <SidebarGroupLabel>Dashboards</SidebarGroupLabel>
          <SidebarMenu>
            <SidebarMenuItem>
              <NavLink to="/" end>
                {({ isActive }) => (
                  <SidebarMenuButton isActive={isActive} tooltip="Overview">
                    <LayoutDashboard />
                    <span>Overview</span>
                  </SidebarMenuButton>
                )}
              </NavLink>
            </SidebarMenuItem>
            <SidebarMenuItem>
              <SidebarMenuButton tooltip="New dashboard" className="text-muted-foreground">
                <Plus />
                <span>New dashboard</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
        </SidebarGroup>
        <SidebarGroup>
          <ExtensionSlot id="sidebar-nav" />
        </SidebarGroup>
      </SidebarContent>
      <ExtensionSlot id="sidebar" />
      <SidebarRail />
    </Sidebar>
  );
}
