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
import { useTranslate, type TranslateFn } from "@nube/starter-ui-core/i18n"

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
  t: TranslateFn,
): string {
  const flowsLabel = t("flow_agent.nav.flows")
  const agentsLabel = t("flow_agent.nav.agents")
  const pagesLabel = t("flow_agent.nav.pages")
  if (pathname.startsWith("/settings")) return t("flow_agent.nav.settings")
  if (pathname.startsWith("/skills")) return t("flow_agent.nav.skills")
  if (pathname.startsWith("/cache-demo")) return t("flow_agent.nav.cache_demo")
  if (pathname.startsWith("/insights/rules"))
    return t("flow_agent.page.insights.rules.title")
  if (pathname.startsWith("/insights/pipelines"))
    return t("flow_agent.page.insights.pipelines.title")
  if (pathname.startsWith("/insights/verdicts/"))
    return t("flow_agent.page.insights.verdicts.title")
  if (pathname.startsWith("/insights/verdicts"))
    return t("flow_agent.page.insights.verdicts.title")
  if (pathname.startsWith("/insights")) return t("flow_agent.nav.insights")
  if (pathname === "/pages/new") return `${pagesLabel} · New page`
  if (pathname.startsWith("/pages/")) {
    const rest = pathname.slice("/pages/".length)
    const id = rest.replace(/\/edit$/, "")
    const p = pages.find((x) => x.id === id)
    const suffix = rest.endsWith("/edit") ? " · Edit" : ""
    return p ? `${pagesLabel} · ${p.name}${suffix}` : pagesLabel
  }
  if (pathname.startsWith("/pages")) return pagesLabel
  if (pathname.startsWith("/agents/")) {
    const id = pathname.slice("/agents/".length)
    const a = agents.find((x) => x.id === id)
    return a ? `${agentsLabel} · ${a.name}` : agentsLabel
  }
  if (pathname.startsWith("/agents")) return agentsLabel
  if (pathname.startsWith("/flows/")) {
    const id = pathname.slice("/flows/".length)
    const f = flows.find((x) => x.id === id)
    return f ? `${flowsLabel} · ${f.name}` : flowsLabel
  }
  return flowsLabel
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
  const t = useTranslate()

  const navMain = useMemo<NavMainItem[]>(
    () => [
      {
        title: t("flow_agent.nav.flows"),
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
        title: t("flow_agent.nav.agents"),
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
        title: t("flow_agent.nav.pages"),
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
        title: t("flow_agent.nav.insights"),
        url: "/insights/rules",
        icon: IconBulb,
        accent: "var(--accent-success)",
        subTestId: "insights-subnav",
        items: [
          { title: t("flow_agent.nav.insights.rules"), url: "/insights/rules", icon: IconFilter },
          { title: t("flow_agent.nav.insights.pipelines"), url: "/insights/pipelines", icon: IconPipeline },
          { title: t("flow_agent.nav.insights.verdicts"), url: "/insights/verdicts", icon: IconGavel },
        ],
      },
      {
        title: t("flow_agent.nav.skills"),
        url: "/skills",
        icon: IconSparkles,
        accent: "var(--accent-success)",
      },
      {
        title: t("flow_agent.nav.cache_demo"),
        url: "/cache-demo",
        icon: IconBolt,
        accent: "var(--accent-info)",
      },
      {
        title: t("flow_agent.nav.settings"),
        url: "/settings",
        icon: IconSettings,
        accent: "var(--accent-settings)",
      },
    ],
    [flows, agents, pages, t],
  )

  const title = titleFor(location.pathname, flows, agents, pages, t)
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
