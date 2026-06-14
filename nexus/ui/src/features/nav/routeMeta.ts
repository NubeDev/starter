import {
  Bell,
  Bot,
  CircleHelp,
  Compass,
  Database,
  Folder,
  History,
  LayoutGrid,
  Radar,
  Shield,
  Sparkles,
  Workflow,
  type LucideIcon,
} from "lucide-react";

import type { StaticRoute } from "@/api/types";

/** Sidebar categories the static routes are bucketed into (presentation only —
 *  access is still gated per node by authz; a category just picks the group a
 *  visible node sits under). `pinned` renders headerless at the very top. */
export type NavCategory = "pinned" | "workspace" | "automation" | "admin";

/** Category section labels + render order. `pinned` has no header. */
export const NAV_CATEGORIES: { key: NavCategory; label: string | null }[] = [
  { key: "pinned", label: null },
  { key: "workspace", label: "Workspace" },
  { key: "automation", label: "Automation" },
  { key: "admin", label: "Admin" },
];

/** Display metadata for each built-in static route (WS-13 §4) — its icon, a
 *  default label, and the sidebar category it belongs to. The closed allow-list
 *  mirrors the router's static pages. */
export const ROUTE_META: Record<
  StaticRoute,
  { label: string; icon: LucideIcon; category: NavCategory }
> = {
  dashboards: { label: "Dashboards", icon: LayoutGrid, category: "pinned" },
  explore: { label: "Explore", icon: Compass, category: "workspace" },
  datasources: { label: "Datasources", icon: Database, category: "workspace" },
  flows: { label: "Flows", icon: Workflow, category: "automation" },
  insights: { label: "Insights", icon: Sparkles, category: "automation" },
  detections: { label: "Detections", icon: Bell, category: "automation" },
  findings: { label: "Findings", icon: Radar, category: "automation" },
  agents: { label: "Agents", icon: Bot, category: "automation" },
  access: { label: "Access", icon: Shield, category: "admin" },
  audit: { label: "Audit", icon: History, category: "admin" },
};

/** The category a top-level nav node renders under. Route nodes use their
 *  route's category; dashboard mounts and groups (user-authored content) sit in
 *  `workspace` alongside Explore/Datasources. */
export function nodeCategory(
  target: { kind: string; route?: StaticRoute },
): NavCategory {
  if (target.kind === "route" && target.route) {
    return routeMeta(target.route).category;
  }
  return "workspace";
}

/** Fallback metadata for a `route` node whose route is not in the allow-list. */
const UNKNOWN_ROUTE_META = {
  label: "Unknown",
  icon: CircleHelp,
  category: "workspace" as NavCategory,
};

/** Resolve a route's display metadata, tolerating an unknown route.
 *
 *  `ROUTE_META` is a closed allow-list, so a `route` node may reference a route
 *  the frontend no longer knows about — e.g. a removed built-in (`alerts` →
 *  `detections`) still seeded in an older tenant's nav tree, or a route from a
 *  stale bundle. A raw `ROUTE_META[route]` lookup returns `undefined` for those,
 *  and dereferencing `.icon`/`.label`/`.category` used to crash the entire
 *  NavTree. This logs a console error and returns a safe placeholder so one
 *  orphaned node degrades gracefully instead of taking down the sidebar. */
export function routeMeta(route: StaticRoute): {
  label: string;
  icon: LucideIcon;
  category: NavCategory;
} {
  const meta = ROUTE_META[route];
  if (!meta) {
    console.error(
      `nav: unknown route "${route}" — not in ROUTE_META (removed or stale nav node); rendering as a placeholder`,
    );
    return UNKNOWN_ROUTE_META;
  }
  return meta;
}

/** The closed list of routes a `route` node may target, for the builder's
 *  picker. Order matches the sidebar's conventional grouping. */
export const STATIC_ROUTES: StaticRoute[] = [
  "dashboards",
  "explore",
  "datasources",
  "flows",
  "insights",
  "detections",
  "findings",
  "agents",
  "access",
  "audit",
];

/** A group node's icon (no target of its own) — a plain folder. */
export const GROUP_ICON = Folder;
