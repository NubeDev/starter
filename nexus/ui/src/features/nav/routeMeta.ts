import {
  Bell,
  Bot,
  Compass,
  Database,
  Folder,
  History,
  LayoutGrid,
  Shield,
  Workflow,
  type LucideIcon,
} from "lucide-react";

import type { StaticRoute } from "@/api/types";

/** Display metadata for each built-in static route (WS-13 §4) — its icon and a
 *  default label. The closed allow-list mirrors the router's static pages. */
export const ROUTE_META: Record<StaticRoute, { label: string; icon: LucideIcon }> = {
  dashboards: { label: "Dashboards", icon: LayoutGrid },
  explore: { label: "Explore", icon: Compass },
  datasources: { label: "Datasources", icon: Database },
  flows: { label: "Flows", icon: Workflow },
  alerts: { label: "Alerts", icon: Bell },
  agents: { label: "Agents", icon: Bot },
  access: { label: "Access", icon: Shield },
  audit: { label: "Audit", icon: History },
};

/** The closed list of routes a `route` node may target, for the builder's
 *  picker. Order matches the sidebar's conventional grouping. */
export const STATIC_ROUTES: StaticRoute[] = [
  "dashboards",
  "explore",
  "datasources",
  "flows",
  "alerts",
  "agents",
  "access",
  "audit",
];

/** A group node's icon (no target of its own) — a plain folder. */
export const GROUP_ICON = Folder;
