import { useMemo, useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { Link } from "react-router-dom"
import { IconBulb } from "@tabler/icons-react"

import { PageHero } from "@/components/page-hero"
import { Badge } from "@/components/ui/badge"
import { Input } from "@/components/ui/input"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { api, type InsightsRule, type InsightsSeverity } from "@/lib/api"
import { useDateFormatters } from "@/hooks/use-date-formatters"
import { useTranslate } from "@nube/starter-ui-core/i18n"

function severityVariant(
  s: InsightsSeverity,
): "default" | "secondary" | "destructive" | "outline" {
  switch (s) {
    case "Critical":
    case "Error":
      return "destructive"
    case "Warn":
      return "default"
    case "Info":
      return "secondary"
    default:
      return "outline"
  }
}

export function RulesList() {
  const rules = useQuery({
    queryKey: ["insights", "rules"],
    queryFn: api.insights.listRules,
  })
  const [filter, setFilter] = useState("")
  const dates = useDateFormatters()
  const tr = useTranslate()

  const filtered = useMemo<InsightsRule[]>(() => {
    const all = rules.data ?? []
    const q = filter.trim().toLowerCase()
    if (!q) return all
    return all.filter(
      (r) =>
        r.id.toLowerCase().includes(q) ||
        r.namespace.toLowerCase().includes(q) ||
        r.summary.toLowerCase().includes(q) ||
        r.tags.some((t) => t.toLowerCase().includes(q)),
    )
  }, [rules.data, filter])

  const total = rules.data?.length ?? 0
  const isEmpty = total === 0 && !rules.isLoading

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <PageHero
        icon={IconBulb}
        accent="var(--accent-success)"
        title={tr("flow_agent.page.insights.rules.title")}
        description={tr("flow_agent.page.insights.rules.description")}
        actions={<Badge variant="secondary">{total} total</Badge>}
      />

      <div className="flex flex-col gap-3">
        <Input
          placeholder={tr("flow_agent.page.insights.rules.filter.placeholder")}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="max-w-md"
        />

        {isEmpty ? (
          <Empty>
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <IconBulb />
              </EmptyMedia>
              <EmptyTitle>No rules yet</EmptyTitle>
              <EmptyDescription>
                The fixture file is empty. Seed{" "}
                <code>examples/flow-agent/fixtures/insights/rules.json</code>.
              </EmptyDescription>
            </EmptyHeader>
            <EmptyContent />
          </Empty>
        ) : (
          <div className="overflow-x-auto rounded-md border">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>ID</TableHead>
                  <TableHead>Kind</TableHead>
                  <TableHead>Namespace</TableHead>
                  <TableHead>Severity</TableHead>
                  <TableHead>Tags</TableHead>
                  <TableHead>Updated</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filtered.map((r) => (
                  <TableRow key={r.id}>
                    <TableCell className="font-mono text-xs">
                      <Link
                        to={`/insights/rules/${encodeURIComponent(r.id)}`}
                        className="hover:underline"
                      >
                        {r.id}
                      </Link>
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className="text-[10px]">
                        {r.kind}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-sm">{r.namespace}</TableCell>
                    <TableCell>
                      <Badge variant={severityVariant(r.severity_default)}>
                        {r.severity_default}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap gap-1">
                        {r.tags.slice(0, 3).map((t) => (
                          <Badge
                            key={t}
                            variant="secondary"
                            className="text-[10px] font-normal"
                          >
                            {t}
                          </Badge>
                        ))}
                        {r.tags.length > 3 && (
                          <span className="text-[10px] text-muted-foreground">
                            +{r.tags.length - 3}
                          </span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {dates.date(r.updated_at)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </div>
    </div>
  )
}
