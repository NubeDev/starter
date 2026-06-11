import type { MouseEvent } from "react";
import { Blocks, LayoutDashboard } from "lucide-react";
import { NavLink, useNavigate } from "react-router-dom";
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
import { useCan } from "@/auth/useCan";
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
  const navigate = useNavigate();
  // Extensions is an admin-only management page. Unlike the nav-tree routes
  // (access-filtered server-side by `GET /api/v1/nav`), this is a static link,
  // so gate it client-side: a non-admin must not even see it in the sidebar —
  // the page itself also returns "Admin only", but a visible-yet-blocked link
  // is the wrong signal. Fails closed while `/me` loads.
  const isAdmin = useCan("admin");
  // Sidebar-nav extension contributions render plain `<a href>` links (the
  // federation host does not share `react-router-dom`, so a remote's own
  // `NavLink` can't see this Router). We intercept clicks bubbling out of the
  // `sidebar-nav` slot and route internal hrefs through the host router for SPA
  // navigation — same pattern rubix uses. External / modified-click / non-left
  // clicks fall through to the browser's default handling.
  function interceptExtensionNav(e: MouseEvent<HTMLDivElement>) {
    if (e.defaultPrevented) return;
    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey)
      return;
    const anchor = (e.target as HTMLElement).closest("a");
    if (!anchor) return;
    const href = anchor.getAttribute("href");
    if (
      !href ||
      href.startsWith("http") ||
      href.startsWith("//") ||
      href.startsWith("#")
    )
      return;
    if (anchor.target && anchor.target !== "_self") return;
    e.preventDefault();
    navigate(href);
  }
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
            admin: isAdmin ? (
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
            ) : undefined,
          }}
        />
        <SidebarGroup onClick={interceptExtensionNav}>
          <ExtensionSlot id="sidebar-nav" />
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarUser />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
