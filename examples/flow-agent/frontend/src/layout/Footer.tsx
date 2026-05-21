import { Link, useLocation } from "react-router-dom"
import { IconChevronRight } from "@tabler/icons-react"

import type { AgentSummary, FlowSummary } from "@/lib/api"
import type { PageRecord } from "@/lib/pages-store"

type Crumb = { label: string; href?: string }

function crumbsFor(
  pathname: string,
  flows: FlowSummary[],
  agents: AgentSummary[],
  pages: PageRecord[],
): Crumb[] {
  if (pathname.startsWith("/settings")) return [{ label: "Settings" }]
  if (pathname.startsWith("/skills")) return [{ label: "Skills" }]

  if (pathname.startsWith("/pages")) {
    const base: Crumb = { label: "Pages", href: "/pages" }
    if (pathname === "/pages" || pathname === "/pages/") {
      return [{ label: "Pages" }]
    }
    if (pathname === "/pages/new") {
      return [base, { label: "New page" }]
    }
    const rest = pathname.slice("/pages/".length)
    const id = rest.replace(/\/edit$/, "")
    const p = pages.find((x) => x.id === id)
    const name = p ? p.name : id
    if (rest.endsWith("/edit")) {
      return [base, { label: name, href: `/pages/${id}` }, { label: "Edit" }]
    }
    return [base, { label: name }]
  }

  if (pathname.startsWith("/agents")) {
    const base: Crumb = { label: "Agents", href: "/agents" }
    if (pathname === "/agents" || pathname === "/agents/") {
      return [{ label: "Agents" }]
    }
    const id = pathname.slice("/agents/".length)
    const a = agents.find((x) => x.id === id)
    return [base, { label: a ? a.name : id }]
  }

  // /flows and default
  const base: Crumb = { label: "Flows", href: "/flows" }
  if (pathname === "/" || pathname === "/flows" || pathname === "/flows/") {
    return [{ label: "Flows" }]
  }
  if (pathname.startsWith("/flows/")) {
    const id = pathname.slice("/flows/".length)
    const f = flows.find((x) => x.id === id)
    return [base, { label: f ? f.name : id }]
  }
  return [{ label: "Flows" }]
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
  const crumbs = crumbsFor(location.pathname, flows, agents, pages)

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
