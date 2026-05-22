import { useMemo } from "react"
import { Outlet, useLocation, useNavigate, NavLink } from "react-router-dom"
import { useQuery, useQueryClient } from "@tanstack/react-query"
import {
  IconSitemap,
  IconRobot,
  IconSettings,
  IconLayoutDashboard,
  IconSparkles,
  IconBulb,
  IconReportAnalytics,
  IconBinaryTree,
  IconMessageCircle,
  IconFile,
  IconFilter,
  IconPipeline,
  IconGavel,
  IconBolt,
} from "@tabler/icons-react"

import { AppSidebar } from "@/components/app-sidebar"
import { SiteHeader } from "@/components/site-header"
import { Footer } from "./Footer"
import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar"
import type { NavMainItem } from "@/components/nav-main"
import { Badge } from "@/components/ui/badge"
import { ThemeToggle } from "@/components/theme-toggle"
import { api, type AgentSummary, type FlowSummary } from "@/lib/api"
import { useSse } from "@/lib/sse"
import { usePages, type PageRecord } from "@/lib/pages-store"

type SidebarEvent =
  | { type: "flow-created"; id: string; name: string }
  | { type: "flow-renamed"; id: string; name: string }
  | { type: "flow-deleted"; id: string }
  | { type: "agent-created"; id: string; name: string }
  | { type: "agent-renamed"; id: string; name: string }
  | { type: "agent-deleted"; id: string }

function titleFor(
  pathname: string,
  flows: FlowSummary[],
  agents: AgentSummary[],
  pages: PageRecord[],
): string {
  if (pathname.startsWith("/settings")) return "Settings"
  if (pathname.startsWith("/skills")) return "Skills"
  if (pathname.startsWith("/cache-demo")) return "Cache demo"
  if (pathname.startsWith("/insights/rules")) return "Insights · Rules"
  if (pathname.startsWith("/insights/pipelines")) return "Insights · Pipelines"
  if (pathname.startsWith("/insights/verdicts/")) return "Insights · Verdict"
  if (pathname.startsWith("/insights/verdicts")) return "Insights · Verdicts"
  if (pathname.startsWith("/insights")) return "Insights"
  if (pathname === "/pages/new") return "Pages · New page"
  if (pathname.startsWith("/pages/")) {
    const rest = pathname.slice("/pages/".length)
    const id = rest.replace(/\/edit$/, "")
    const p = pages.find((x) => x.id === id)
    const suffix = rest.endsWith("/edit") ? " · Edit" : ""
    return p ? `Pages · ${p.name}${suffix}` : "Pages"
  }
  if (pathname.startsWith("/pages")) return "Pages"
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
  if (pathname.startsWith("/skills")) return "/skills"
  if (pathname.startsWith("/cache-demo")) return "/cache-demo"
  if (pathname.startsWith("/insights/verdicts")) return "/insights/verdicts"
  if (pathname.startsWith("/insights/pipelines")) return "/insights/pipelines"
  if (pathname.startsWith("/insights/rules")) return "/insights/rules"
  if (pathname.startsWith("/insights")) return "/insights/rules"
  if (pathname === "/pages/new") return "/pages"
  if (pathname.startsWith("/pages/")) {
    const rest = pathname.slice("/pages/".length).replace(/\/edit$/, "")
    return `/pages/${rest}`
  }
  if (pathname.startsWith("/pages")) return "/pages"
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
  const pages = usePages()

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
          icon: IconBinaryTree,
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
          icon: IconMessageCircle,
        })),
      },
      {
        title: "Pages",
        url: "/pages",
        icon: IconLayoutDashboard,
        accent: "var(--accent-info)",
        subTestId: "pages-subnav",
        items: pages.slice(0, 12).map((p) => ({
          title: p.name,
          url: `/pages/${p.id}`,
          icon: IconFile,
        })),
      },
      {
        title: "Insights",
        url: "/insights/rules",
        icon: IconBulb,
        accent: "var(--accent-success)",
        subTestId: "insights-subnav",
        items: [
          { title: "Rules", url: "/insights/rules", icon: IconFilter },
          { title: "Pipelines", url: "/insights/pipelines", icon: IconPipeline },
          { title: "Verdicts", url: "/insights/verdicts", icon: IconGavel },
        ],
      },
      {
        title: "Skills",
        url: "/skills",
        icon: IconSparkles,
        accent: "var(--accent-success)",
      },
      {
        title: "Cache demo",
        url: "/cache-demo",
        icon: IconBolt,
        accent: "var(--accent-info)",
      },
      {
        title: "Settings",
        url: "/settings",
        icon: IconSettings,
        accent: "var(--accent-settings)",
      },
    ],
    [flows, agents, pages],
  )

  const title = titleFor(location.pathname, flows, agents, pages)
  const activeUrl = activeUrlFor(location.pathname)

  return (
    <div className="min-h-dvh bg-background text-foreground pb-8">
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
      <div className="fixed inset-x-0 bottom-0 z-30">
        <Footer flows={flows} agents={agents} pages={pages} />
      </div>
    </div>
  )
}

// Keep NavLink referenced for type clarity in pages.
export { NavLink }
