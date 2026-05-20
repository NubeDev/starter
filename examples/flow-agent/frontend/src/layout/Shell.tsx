import { useMemo } from "react"
import { Outlet, useLocation, useNavigate, NavLink } from "react-router-dom"
import { useQuery, useQueryClient } from "@tanstack/react-query"
import {
  IconSitemap,
  IconRobot,
  IconSettings,
} from "@tabler/icons-react"

import { AppSidebar } from "@/components/app-sidebar"
import { SiteHeader } from "@/components/site-header"
import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar"
import type { NavMainItem } from "@/components/nav-main"
import { Badge } from "@/components/ui/badge"
import { ThemeToggle } from "@/components/theme-toggle"
import { api, type AgentSummary, type FlowSummary } from "@/lib/api"
import { useSse } from "@/lib/sse"

type SidebarEvent =
  | { type: "flow-created"; id: string; name: string }
  | { type: "flow-renamed"; id: string; name: string }
  | { type: "flow-deleted"; id: string }
  | { type: "agent-created"; id: string; name: string }
  | { type: "agent-renamed"; id: string; name: string }
  | { type: "agent-deleted"; id: string }

function titleFor(pathname: string, flows: FlowSummary[], agents: AgentSummary[]): string {
  if (pathname.startsWith("/settings")) return "Settings"
  if (pathname.startsWith("/agents/")) {
    const id = pathname.slice("/agents/".length)
    const a = agents.find((x) => x.id === id)
    return a ? `Agents · ${a.name}` : "Agents"
  }
  if (pathname.startsWith("/agents")) return "Agents"
  if (pathname.startsWith("/flows/")) {
    const id = pathname.slice("/flows/".length)
    const f = flows.find((x) => x.id === id)
    return f ? `Flows · ${f.name}` : "Flows"
  }
  return "Flows"
}

function activeUrlFor(pathname: string): string {
  if (pathname.startsWith("/settings")) return "/settings"
  if (pathname.startsWith("/agents/")) return pathname
  if (pathname.startsWith("/agents")) return "/agents"
  if (pathname.startsWith("/flows/")) return pathname
  return "/flows"
}

export function Shell() {
  const qc = useQueryClient()
  const location = useLocation()
  const navigate = useNavigate()

  const flowsQ = useQuery({ queryKey: ["flows"], queryFn: api.flows.list })
  const agentsQ = useQuery({ queryKey: ["agents"], queryFn: api.agents.list })

  useSse<SidebarEvent>("/api/events", (ev) => {
    if (ev.type.startsWith("flow-")) {
      qc.invalidateQueries({ queryKey: ["flows"] })
    } else if (ev.type.startsWith("agent-")) {
      qc.invalidateQueries({ queryKey: ["agents"] })
    }
  })

  const flows = flowsQ.data ?? []
  const agents = agentsQ.data ?? []

  const navMain = useMemo<NavMainItem[]>(
    () => [
      {
        title: "Flows",
        url: "/flows",
        icon: IconSitemap,
        accent: "var(--accent-flows)",
        subTestId: "flows-subnav",
        items: flows.slice(0, 12).map((f) => ({
          title: f.name,
          url: `/flows/${f.id}`,
          badge: (
            <Badge
              variant="outline"
              className="h-5 px-1.5 text-[10px] font-normal text-muted-foreground"
            >
              v{f.version}
            </Badge>
          ),
        })),
      },
      {
        title: "Agents",
        url: "/agents",
        icon: IconRobot,
        accent: "var(--accent-agents)",
        subTestId: "agents-subnav",
        items: agents.slice(0, 12).map((a) => ({
          title: a.name,
          url: `/agents/${a.id}`,
        })),
      },
      {
        title: "Settings",
        url: "/settings",
        icon: IconSettings,
        accent: "var(--accent-settings)",
      },
    ],
    [flows, agents],
  )

  const title = titleFor(location.pathname, flows, agents)
  const activeUrl = activeUrlFor(location.pathname)

  return (
    <div className="min-h-dvh bg-background text-foreground">
      <SidebarProvider>
        <AppSidebar
          navMain={navMain}
          activeUrl={activeUrl}
          user={{ name: "operator", email: "" }}
          onNavigate={(url) => navigate(url)}
          brand={{ title: "flow-agent", url: "/flows" }}
        />
        <SidebarInset>
          <SiteHeader title={title} actions={<ThemeToggle />} />
          <div className="flex flex-1 flex-col">
            <div className="@container/main flex flex-1 flex-col">
              <Outlet />
            </div>
          </div>
        </SidebarInset>
      </SidebarProvider>
    </div>
  )
}

// Keep NavLink referenced for type clarity in pages.
export { NavLink }
