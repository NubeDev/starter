import {
  Bell,
  Bot,
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
  alerts: { label: "Alerts", icon: Bell, category: "automation" },
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
    return ROUTE_META[target.route].category;
  }
  return "workspace";
}

/** The closed list of routes a `route` node may target, for the builder's
 *  picker. Order matches the sidebar's conventional grouping. */
export const STATIC_ROUTES: StaticRoute[] = [
  "dashboards",
  "explore",
  "datasources",
  "flows",
  "insights",
  "alerts",
  "findings",
  "agents",
  "access",
  "audit",
];

/** A group node's icon (no target of its own) — a plain folder. */
export const GROUP_ICON = Folder;
