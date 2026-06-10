import { Blocks, LayoutDashboard } from "lucide-react";
import { NavLink } from "react-router-dom";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";

import { useLayout } from "@/app/LayoutProvider";
import { SidebarUser } from "@/app/SidebarUser";
import { ExtensionSlot } from "@/extensions/ExtensionSlot";
import { SidebarStarred } from "@/features/dashboards/SidebarStarred";
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
        {/* The caller's starred dashboards, pinned above the nav tree for quick
            access. Per-user favourites; renders nothing when none are starred. */}
        <SidebarStarred />
        {/* The primary navigation is the access-filtered nav tree (WS-13): one
            place that lists dashboard mounts and static pages, gated per node.
            It replaces the old hardcoded Dashboards/Data/Manage groups — the
            static pages now live as `route` nodes seeded into the tree. */}
        {/* Primary navigation — the access-filtered nav tree, rendered as
            categorised sections (Dashboards pinned, then Workspace / Automation
            / Admin). Building and organising it (and granting access per node)
            now lives under Access → Navigation, so there's no edit affordance
            here. */}
        {/* Extensions admin (WS-14) — a static link, not a nav-tree route node,
            but admin-gated like Access/Audit. Dropped into the tree's Admin
            group via `extras` so it sits flush with Access/Audit (a separate
            group would add its own spacing and float detached). */}
        <NavTree
          extras={{
            admin: (
              <SidebarMenuItem>
                <NavLink to="/extensions" end>
                  {({ isActive }) => (
                    <SidebarMenuButton
                      isActive={isActive}
                      tooltip="Extensions"
                      className="text-muted-foreground"
                    >
                      <Blocks />
                      <span>Extensions</span>
                    </SidebarMenuButton>
                  )}
                </NavLink>
              </SidebarMenuItem>
            ),
          }}
        />
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
