import { useState } from "react";
import { ChevronRight } from "lucide-react";
import { NavLink } from "react-router-dom";

import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
} from "@/components/ui/sidebar";
import { dashboardIcon } from "@/features/dashboards/appearance";
import { useDashboards } from "@/features/dashboards/useDashboards";
import { navNodeHref, type NavTreeNode } from "@/features/nav/navTree";
import { GROUP_ICON, ROUTE_META } from "@/features/nav/routeMeta";
import { useNavTree } from "@/features/nav/useNavTree";

// The primary sidebar navigation (WS-13 §4) — the access-filtered nav tree.
// Group nodes expand/collapse; dashboard/route nodes are links. A dashboard
// node opens `d/:slug?nav=:id` so the page binds the node's context; a route
// node opens its static page. Replaces the hardcoded SidebarDashboards.
export function NavTree() {
  const { tree, isPending, isError } = useNavTree();
  // Resolve a dashboard target's id → slug for its link. The dashboards list is
  // already cached by the shell, so this adds no round-trip.
  const { data: dashboards } = useDashboards();
  const slugOf = (id: string) =>
    dashboards?.find((d) => d.id === id)?.slug;

  if (isPending) {
    return (
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton disabled className="text-muted-foreground">
            <GROUP_ICON />
            <span>Loading…</span>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    );
  }
  if (isError || tree.length === 0) {
    return (
      <SidebarMenu>
        <SidebarMenuItem>
          <SidebarMenuButton disabled className="text-muted-foreground">
            <GROUP_ICON />
            <span>{isError ? "Unavailable" : "No navigation yet"}</span>
          </SidebarMenuButton>
        </SidebarMenuItem>
      </SidebarMenu>
    );
  }

  return (
    <SidebarMenu>
      {tree.map((n) => (
        <NavTreeItem key={n.id} node={n} slugOf={slugOf} />
      ))}
    </SidebarMenu>
  );
}

function NavTreeItem({
  node,
  slugOf,
}: {
  node: NavTreeNode;
  slugOf: (id: string) => string | undefined;
}) {
  const [open, setOpen] = useState(true);

  if (node.target.kind === "group") {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton
          onClick={() => setOpen((o) => !o)}
          tooltip={node.title}
        >
          <ChevronRight
            className="transition-transform"
            style={{ transform: open ? "rotate(90deg)" : undefined }}
          />
          <span>{node.title}</span>
        </SidebarMenuButton>
        {open && node.children.length > 0 ? (
          <SidebarMenuSub>
            {node.children.map((c) => (
              <NavTreeItem key={c.id} node={c} slugOf={slugOf} />
            ))}
          </SidebarMenuSub>
        ) : null}
      </SidebarMenuItem>
    );
  }

  const href = navNodeHref(node, slugOf);
  const Icon =
    node.target.kind === "route"
      ? ROUTE_META[node.target.route].icon
      : dashboardIcon(node.icon ?? "Activity");

  // A dashboard target whose page is gone (swept) renders disabled rather than
  // a dead link — the node survives a deleted page (WS-13 §1).
  if (!href) {
    return (
      <SidebarMenuItem>
        <SidebarMenuButton disabled tooltip={node.title}>
          <Icon />
          <span>{node.title}</span>
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  }

  return (
    <SidebarMenuItem>
      <NavLink to={href} end>
        {({ isActive }) => (
          <SidebarMenuButton
            isActive={isActive}
            tooltip={node.title}
            style={
              node.accent ? { color: `hsl(${node.accent})` } : undefined
            }
          >
            <Icon />
            <span>{node.title}</span>
          </SidebarMenuButton>
        )}
      </NavLink>
    </SidebarMenuItem>
  );
}
