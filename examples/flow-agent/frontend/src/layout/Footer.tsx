import { Link, useLocation } from "react-router-dom"
import { IconChevronRight } from "@tabler/icons-react"
import { useTranslate, type TranslateFn } from "@nube/starter-ui-core/i18n"

import type { AgentSummary, FlowSummary } from "@/lib/api"
import type { PageRecord } from "@/lib/pages-store"

type Crumb = { label: string; href?: string }

function crumbsFor(
  pathname: string,
  flows: FlowSummary[],
  agents: AgentSummary[],
  pages: PageRecord[],
  t: TranslateFn,
): Crumb[] {
  const flowsLabel = t("flow_agent.nav.flows")
  const agentsLabel = t("flow_agent.nav.agents")
  const pagesLabel = t("flow_agent.nav.pages")

  if (pathname.startsWith("/settings"))
    return [{ label: t("flow_agent.nav.settings") }]
  if (pathname.startsWith("/skills"))
    return [{ label: t("flow_agent.nav.skills") }]

  if (pathname.startsWith("/pages")) {
    const base: Crumb = { label: pagesLabel, href: "/pages" }
    if (pathname === "/pages" || pathname === "/pages/") {
      return [{ label: pagesLabel }]
    }
    if (pathname === "/pages/new") {
      return [base, { label: t("flow_agent.breadcrumb.new_page") }]
    }
    const rest = pathname.slice("/pages/".length)
    const id = rest.replace(/\/edit$/, "")
    const p = pages.find((x) => x.id === id)
    const name = p ? p.name : id
    if (rest.endsWith("/edit")) {
      return [
        base,
        { label: name, href: `/pages/${id}` },
        { label: t("flow_agent.breadcrumb.edit") },
      ]
    }
    return [base, { label: name }]
  }

  if (pathname.startsWith("/agents")) {
    const base: Crumb = { label: agentsLabel, href: "/agents" }
    if (pathname === "/agents" || pathname === "/agents/") {
      return [{ label: agentsLabel }]
    }
    const id = pathname.slice("/agents/".length)
    const a = agents.find((x) => x.id === id)
    return [base, { label: a ? a.name : id }]
  }

  // /flows and default
  const base: Crumb = { label: flowsLabel, href: "/flows" }
  if (pathname === "/" || pathname === "/flows" || pathname === "/flows/") {
    return [{ label: flowsLabel }]
  }
  if (pathname.startsWith("/flows/")) {
    const id = pathname.slice("/flows/".length)
    const f = flows.find((x) => x.id === id)
    return [base, { label: f ? f.name : id }]
  }
  return [{ label: flowsLabel }]
}

export function Footer({
  flows,
  agents,
  pages,
}: {
  flows: FlowSummary[]
  agents: AgentSummary[]
  pages: PageRecord[]
}) {
  const location = useLocation()
  const t = useTranslate()
  const crumbs = crumbsFor(location.pathname, flows, agents, pages, t)

  return (
    <footer
      className="flex h-8 shrink-0 items-center justify-between gap-3 border-t border-border/60 bg-card/60 px-3 text-[11px]"
      data-testid="app-footer"
    >
      <nav
        aria-label="Breadcrumb"
        className="min-w-0 flex-1 truncate"
      >
        <ol className="flex items-center gap-1 text-xs text-muted-foreground">
          {crumbs.map((c, i) => {
            const isLast = i === crumbs.length - 1
            return (
              <li key={`${c.label}-${i}`} className="flex items-center gap-1">
                {c.href && !isLast ? (
                  <Link
                    to={c.href}
                    className="hover:text-foreground transition-colors"
                  >
                    {c.label}
                  </Link>
                ) : (
                  <span
                    className={isLast ? "text-foreground" : ""}
                    aria-current={isLast ? "page" : undefined}
                  >
                    {c.label}
                  </span>
                )}
                {!isLast ? (
                  <IconChevronRight
                    className="size-3 text-muted-foreground/60"
                    aria-hidden
                  />
                ) : null}
              </li>
            )
          })}
        </ol>
      </nav>
    </footer>
  )
}
