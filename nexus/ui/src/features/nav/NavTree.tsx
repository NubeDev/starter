import { useState } from "react";
import { ChevronRight } from "lucide-react";
import { NavLink } from "react-router-dom";

import {
  SidebarGroup,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
} from "@/components/ui/sidebar";
import { dashboardIcon } from "@/features/dashboards/appearance";
import { useDashboards } from "@/features/dashboards/useDashboards";
import { navNodeHref, type NavTreeNode } from "@/features/nav/navTree";
import {
  GROUP_ICON,
  NAV_CATEGORIES,
  type NavCategory,
  nodeCategory,
  ROUTE_META,
} from "@/features/nav/routeMeta";
import { useNavTree } from "@/features/nav/useNavTree";

// The primary sidebar navigation (WS-13 §4) — the access-filtered nav tree.
// Group nodes expand/collapse; dashboard/route nodes are links. A dashboard
// node opens `d/:slug?nav=:id` so the page binds the node's context; a route
// node opens its static page. Replaces the hardcoded SidebarDashboards.
//
// `extras` lets the shell drop static menu items into a category's group (keyed
// by category) so they sit flush with the tree's items — used for admin links
// like Extensions that aren't tree nodes but belong under Admin.
export function NavTree({
  extras,
}: {
  extras?: Partial<Record<NavCategory, React.ReactNode>>;
} = {}) {
  const { tree, isPending, isError } = useNavTree();
  // Resolve a dashboard target's id → slug for its link. The dashboards list is
  // already cached by the shell, so this adds no round-trip.
  const { data: dashboards } = useDashboards();
  const slugOf = (id: string) =>
    dashboards?.find((d) => d.id === id)?.slug;

  if (isPending || isError || tree.length === 0) {
    return (
      <SidebarGroup>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton disabled className="text-muted-foreground">
              <GROUP_ICON />
              <span>
                {isPending
                  ? "Loading…"
                  : isError
                    ? "Unavailable"
                    : "No navigation yet"}
              </span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarGroup>
    );
  }

  // Bucket the top-level nodes into sidebar categories (presentation only —
  // each node is already access-filtered). A node's children stay nested under
  // it; only the roots are categorised. A category renders when it has nodes or
  // `extras` (static links a caller drops into a category's menu, e.g.
  // Extensions under Admin) so the extras share the group — and its spacing —
  // with the tree nodes instead of floating in a detached group below.
  return (
    <>
      {NAV_CATEGORIES.map(({ key, label }) => {
        const nodes = tree.filter((n) => nodeCategory(n.target) === key);
        const extra = extras?.[key];
        if (nodes.length === 0 && !extra) return null;
        return (
          <SidebarGroup key={key}>
            {label ? <SidebarGroupLabel>{label}</SidebarGroupLabel> : null}
            <SidebarMenu>
              {nodes.map((n) => (
                <NavTreeItem key={n.id} node={n} slugOf={slugOf} />
              ))}
              {extra}
            </SidebarMenu>
          </SidebarGroup>
        );
      })}
    </>
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
  // A `route` node may reference a route the frontend no longer knows about (a
  // removed built-in still seeded in an older tenant's tree, or a stale bundle).
  // `ROUTE_META` is a closed allow-list, so fall back to a generic icon rather
  // than crash when the route is unknown — matching `nodeCategory`'s posture.
  const Icon =
    node.target.kind === "route"
      ? (ROUTE_META[node.target.route]?.icon ?? dashboardIcon("Activity"))
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
