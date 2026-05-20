// Nested sidebar: top-level `Flows` and `Agents` groups expand into
// live lists fed by react-query + the existing `/api/events` SSE
// channel. Expand state + active section persist via `useUiStore`
// (single `fa-ui` localStorage key).

import { useEffect, useMemo } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { NavLink, useLocation } from "react-router-dom";
import {
  Sidebar as SidebarShell,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarItem,
  SidebarTree,
  type SidebarTreeNode,
} from "@nube/starter-ui-kit";

import { api, type AgentSummary, type FlowSummary } from "../lib/api";
import { useSse } from "../lib/sse";
import { useUiStore } from "../state/ui-store";

type SidebarEvent =
  | { type: "flow-created"; id: string; name: string }
  | { type: "flow-renamed"; id: string; name: string }
  | { type: "flow-deleted"; id: string }
  | { type: "agent-created"; id: string; name: string }
  | { type: "agent-renamed"; id: string; name: string }
  | { type: "agent-deleted"; id: string };

export function Sidebar() {
  const qc = useQueryClient();
  const location = useLocation();
  const expanded = useUiStore((s) => s.expandedGroups);
  const setExpanded = useUiStore((s) => s.setExpandedGroups);
  const toggleGroup = useUiStore((s) => s.toggleGroup);
  const setActiveSection = useUiStore((s) => s.setActiveSection);

  const flows = useQuery({ queryKey: ["flows"], queryFn: api.flows.list });
  const agents = useQuery({ queryKey: ["agents"], queryFn: api.agents.list });

  // SSE: re-fetch on any sidebar event. Cheap, atomic, no manual
  // cache surgery.
  useSse<SidebarEvent>("/api/events", (ev) => {
    if (ev.type.startsWith("flow-")) {
      qc.invalidateQueries({ queryKey: ["flows"] });
    } else if (ev.type.startsWith("agent-")) {
      qc.invalidateQueries({ queryKey: ["agents"] });
    }
  });

  // Keep activeSection in sync with the current route.
  useEffect(() => {
    if (location.pathname.startsWith("/agents")) setActiveSection("agents");
    else if (location.pathname.startsWith("/settings"))
      setActiveSection("settings");
    else setActiveSection("flows");
  }, [location.pathname, setActiveSection]);

  const expandedSet = useMemo(() => new Set(expanded), [expanded]);

  const flowsTree = useMemo<SidebarTreeNode[]>(
    () => buildFlowsTree(flows.data ?? [], location.pathname),
    [flows.data, location.pathname],
  );

  const agentsTree = useMemo<SidebarTreeNode[]>(
    () => buildAgentsTree(agents.data ?? [], location.pathname),
    [agents.data, location.pathname],
  );

  return (
    <SidebarShell className="hidden md:flex">
      <SidebarHeader>
        <div className="size-5 rounded-md bg-primary/90" aria-hidden />
        <span className="truncate">flow-agent</span>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup
          open={expandedSet.has("flows")}
          onOpenChange={() => toggleGroup("flows")}
        >
          <SidebarGroupLabel>Flows</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarTree
              nodes={flowsTree}
              expanded={expandedSet}
              onExpandedChange={(next) => setExpanded(Array.from(next))}
            />
            {flowsTree.length === 0 && (
              <span className="px-2 py-1 text-xs text-muted-foreground">
                {flows.isLoading ? "Loading…" : "No flows"}
              </span>
            )}
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup
          open={expandedSet.has("agents")}
          onOpenChange={() => toggleGroup("agents")}
        >
          <SidebarGroupLabel>Agents</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarTree
              nodes={agentsTree}
              expanded={expandedSet}
              onExpandedChange={(next) => setExpanded(Array.from(next))}
            />
            {agentsTree.length === 0 && (
              <span className="px-2 py-1 text-xs text-muted-foreground">
                {agents.isLoading ? "Loading…" : "No agents"}
              </span>
            )}
          </SidebarGroupContent>
        </SidebarGroup>

        <div className="mt-3 border-t border-border/60 pt-3">
          <SidebarItem
            asChild
            active={location.pathname.startsWith("/settings")}
          >
            <NavLink to="/settings">Settings</NavLink>
          </SidebarItem>
        </div>
      </SidebarContent>
    </SidebarShell>
  );
}

function buildFlowsTree(
  flows: FlowSummary[],
  pathname: string,
): SidebarTreeNode[] {
  return flows.map((f) => ({
    id: `flow/${f.id}`,
    label: f.name,
    active: pathname === `/flows/${f.id}`,
    render: (inner) => (
      <NavLink key={f.id} to={`/flows/${f.id}`} className="block">
        {inner}
      </NavLink>
    ),
  }));
}

function buildAgentsTree(
  agents: AgentSummary[],
  pathname: string,
): SidebarTreeNode[] {
  return agents.map((a) => ({
    id: `agent/${a.id}`,
    label: a.name,
    active: pathname === `/agents/${a.id}`,
    render: (inner) => (
      <NavLink key={a.id} to={`/agents/${a.id}`} className="block">
        {inner}
      </NavLink>
    ),
  }));
}
