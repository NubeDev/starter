// `/pages` — list of AI-built SDUI pages saved to `localStorage`
// under `flow-agent:pages`. Subscribes to the in-process pub/sub +
// cross-tab `storage` event via `usePages()` so the sidebar and this
// list stay in sync without manual invalidations.

import { Link, useNavigate } from "react-router-dom"
import { IconPlus, IconLayoutDashboard } from "@tabler/icons-react"

import { Button } from "@/components/ui/button"
import { PageHero } from "@/components/page-hero"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { usePages, type PageRecord } from "@/lib/pages-store"
import { useDateFormatters } from "@/hooks/use-date-formatters"
import { useTranslate, type TranslateFn } from "@nube/starter-ui-core/i18n"

function summary(p: PageRecord, t: TranslateFn): string {
  // Sniff the first row of children to produce a one-liner blurb. The
  // tree is the wire-format UiComponentTree (root + children), so we
  // walk one level deep — enough for "4 KPIs · table"-style hints
  // without dragging in a real description field.
  const root = (p.tree as { root?: { children?: Array<{ type?: string }> } })
    .root
  const kinds = root?.children?.map((c) => c.type ?? "node") ?? []
  if (kinds.length === 0) return t("flow_agent.pages.empty_summary")
  const counts = new Map<string, number>()
  for (const k of kinds) counts.set(k, (counts.get(k) ?? 0) + 1)
  return [...counts.entries()]
    .map(([k, n]) => (n > 1 ? `${n} ${k}s` : k))
    .join(" · ")
}

function relative(
  ts: number,
  formatDate: (ts: number) => string,
  t: TranslateFn,
): string {
  const diff = Date.now() - ts
  const m = Math.floor(diff / 60_000)
  if (m < 1) return t("flow_agent.time.just_now")
  if (m < 60) return t("flow_agent.time.minutes_ago", { n: m })
  const h = Math.floor(m / 60)
  if (h < 24) return t("flow_agent.time.hours_ago", { n: h })
  const d = Math.floor(h / 24)
  if (d < 7) return t("flow_agent.time.days_ago", { n: d })
  return t("flow_agent.time.updated_on", { date: formatDate(ts) })
}

export function PagesList() {
  const pages = usePages()
  const navigate = useNavigate()
  const dates = useDateFormatters()
  const t = useTranslate()
  const total = pages.length
  const isEmpty = total === 0

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <PageHero
        icon={IconLayoutDashboard}
        accent="var(--accent-info)"
        title={t("flow_agent.page.pages.title")}
        description={t("flow_agent.page.pages.description")}
        actions={
          <>
            <Badge variant="secondary">{total} total</Badge>
            <Button onClick={() => navigate("/pages/new")}>
              <IconPlus className="size-4" />
              {t("flow_agent.action.new_page")}
            </Button>
          </>
        }
      />

      {isEmpty ? (
        <Empty className="border border-dashed bg-card/30">
          <EmptyHeader>
            <EmptyMedia variant="icon" aria-hidden>
              <IconLayoutDashboard className="size-5" />
            </EmptyMedia>
            <EmptyTitle>{t("flow_agent.page.pages.empty.title")}</EmptyTitle>
            <EmptyDescription>
              {t("flow_agent.page.pages.empty.description")}
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={() => navigate("/pages/new")}>
              <IconPlus className="size-4" />
              {t("flow_agent.action.new_page")}
            </Button>
          </EmptyContent>
        </Empty>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {pages.map((p) => (
            <Card key={p.id} className="card-lift flex flex-col">
              <CardHeader>
                <CardTitle className="text-base">
                  <Link to={`/pages/${p.id}`} className="hover:underline">
                    {p.name}
                  </Link>
                </CardTitle>
                <CardDescription>{summary(p, t)}</CardDescription>
              </CardHeader>
              <CardContent className="flex-1 text-xs text-muted-foreground">
                {relative(p.updatedAt, dates.date, t)}
              </CardContent>
              <CardFooter className="gap-2">
                <Button asChild size="sm" variant="secondary">
                  <Link to={`/pages/${p.id}`}>{t("flow_agent.action.view")}</Link>
                </Button>
                <Button asChild size="sm" variant="outline">
                  <Link to={`/pages/${p.id}/edit`}>{t("flow_agent.action.edit")}</Link>
                </Button>
              </CardFooter>
            </Card>
          ))}
        </div>
      )}
    </div>
  )
}
