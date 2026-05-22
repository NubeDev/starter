import { useMemo, useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { useParams, Link } from "react-router-dom"
import { IconReportAnalytics, IconArrowLeft } from "@tabler/icons-react"

import { PageHero } from "@/components/page-hero"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
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
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import {
  api,
  type InsightsSeverity,
  type InsightsVerdict,
} from "@/lib/api"
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

export function VerdictsView() {
  const { id } = useParams<{ id?: string }>()
  return id ? <VerdictDetail id={id} /> : <VerdictsListPanel />
}

function VerdictsListPanel() {
  const verdicts = useQuery({
    queryKey: ["insights", "verdicts"],
    queryFn: () => api.insights.listVerdicts(),
  })
  const [filter, setFilter] = useState("")
  const dates = useDateFormatters()
  const tr = useTranslate()

  const filtered = useMemo<InsightsVerdict[]>(() => {
    const all = verdicts.data ?? []
    const q = filter.trim().toLowerCase()
    if (!q) return all
    return all.filter(
      (v) =>
        v.rule_id.toLowerCase().includes(q) ||
        v.severity.toLowerCase().includes(q) ||
        v.summary.toLowerCase().includes(q) ||
        v.tags.some((t) => t.toLowerCase().includes(q)),
    )
  }, [verdicts.data, filter])

  const total = verdicts.data?.length ?? 0
  const isEmpty = total === 0 && !verdicts.isLoading

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <PageHero
        icon={IconReportAnalytics}
        accent="var(--accent-info)"
        title={tr("flow_agent.page.insights.verdicts.title")}
        description={tr("flow_agent.page.insights.verdicts.description")}
        actions={<Badge variant="secondary">{total} total</Badge>}
      />

      <Input
        placeholder={tr("flow_agent.page.insights.verdicts.filter.placeholder")}
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        className="max-w-md"
      />

      {isEmpty ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <IconReportAnalytics />
            </EmptyMedia>
            <EmptyTitle>No verdicts</EmptyTitle>
            <EmptyDescription>
              Fixtures empty — see{" "}
              <code>fixtures/insights/verdicts.json</code>.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="overflow-x-auto rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>ID</TableHead>
                <TableHead>Rule</TableHead>
                <TableHead>At</TableHead>
                <TableHead>Severity</TableHead>
                <TableHead>Confidence</TableHead>
                <TableHead>Summary</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.map((v) => (
                <TableRow key={v.id}>
                  <TableCell className="font-mono text-xs">
                    <Link
                      to={`/insights/verdicts/${encodeURIComponent(v.id)}`}
                      className="hover:underline"
                    >
                      {v.id}
                    </Link>
                  </TableCell>
                  <TableCell className="font-mono text-xs">
                    {v.rule_id}
                  </TableCell>
                  <TableCell className="text-xs text-muted-foreground">
                    {dates.dateTime(v.at)}
                  </TableCell>
                  <TableCell>
                    <div className="flex items-center gap-1">
                      <Badge variant={severityVariant(v.severity)}>
                        {v.severity}
                      </Badge>
                      {v.coverage.quality_flags.some((f) =>
                        f.id.includes("retroactive-correction"),
                      ) && (
                        <Badge
                          variant="outline"
                          className="text-[10px]"
                          title="starter.quality.retroactive-correction@1"
                        >
                          retro
                        </Badge>
                      )}
                    </div>
                  </TableCell>
                  <TableCell className="text-xs">
                    {(v.coverage.effective.confidence * 100).toFixed(0)}%
                  </TableCell>
                  <TableCell className="max-w-md truncate text-sm">
                    {v.summary}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  )
}

function VerdictDetail({ id }: { id: string }) {
  const verdict = useQuery({
    queryKey: ["insights", "verdict", id],
    queryFn: () => api.insights.getVerdict(id),
  })
  const dates = useDateFormatters()

  if (verdict.isLoading) {
    return <div className="px-4 py-6 lg:px-6">Loading…</div>
  }
  if (verdict.error || !verdict.data) {
    return (
      <div className="px-4 py-6 lg:px-6">
        <p className="text-sm text-destructive">Verdict not found.</p>
        <Link to="/insights/verdicts" className="text-sm underline">
          Back to verdicts
        </Link>
      </div>
    )
  }
  const v = verdict.data
  const retroFlag = v.coverage.quality_flags.find((f) =>
    f.id.includes("retroactive-correction"),
  )

  return (
    <article className="verdict-print flex flex-col gap-6 px-4 py-6 lg:px-6">
      {/* Screen-only back link / actions (hidden by print stylesheet, S4). */}
      <div className="flex items-center justify-between print:hidden">
        <Button variant="ghost" size="sm" asChild>
          <Link to="/insights/verdicts">
            <IconArrowLeft className="mr-1 size-4" />
            Back
          </Link>
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => window.print()}
        >
          Print / Save PDF
        </Button>
      </div>

      {/* Print-only header — hidden on screen, prominent on paper (S4). */}
      <header className="hidden print:block">
        <h1 className="text-xl font-bold">Verdict — {v.rule_id}</h1>
        <p className="text-xs">
          {v.id} · generated {new Date().toISOString()}
        </p>
      </header>

      <PageHero
        icon={IconReportAnalytics}
        accent="var(--accent-info)"
        title={`Verdict ${v.id}`}
        description={v.summary}
        actions={
          <div className="flex items-center gap-2">
            <Badge variant={severityVariant(v.severity)}>{v.severity}</Badge>
            {retroFlag && (
              <Badge variant="outline">retroactive-correction</Badge>
            )}
          </div>
        }
      />

      <section className="grid gap-2 text-sm sm:grid-cols-2">
        <Field label="Rule" value={v.rule_id} mono />
        <Field
          label="At"
          value={`${dates.dateTime(v.at)} (${v.tz})`}
        />
        <Field
          label="Window"
          value={`${dates.dateTime(v.window.start)} → ${dates.dateTime(v.window.end)}`}
        />
        <Field
          label="Confidence (raw / effective)"
          value={`${(v.coverage.raw.confidence * 100).toFixed(
            0,
          )}% / ${(v.coverage.effective.confidence * 100).toFixed(0)}%`}
        />
        <Field
          label="Samples"
          value={`${v.coverage.raw.samples_present} / ${v.coverage.raw.samples_expected}`}
        />
        {v.correlation_id && (
          <Field label="Correlation" value={v.correlation_id} mono />
        )}
      </section>

      <section>
        <h2 className="mb-2 text-sm font-semibold">Tags</h2>
        <div className="flex flex-wrap gap-1">
          {v.tags.map((t) => (
            <Badge key={t} variant="secondary" className="text-[10px]">
              {t}
            </Badge>
          ))}
        </div>
      </section>

      <section>
        <h2 className="mb-2 text-sm font-semibold">Evidence</h2>
        {renderEvidenceTable(v.evidence)}
      </section>

      {v.coverage.quality_flags.length > 0 && (
        <section>
          <h2 className="mb-2 text-sm font-semibold">Quality flags</h2>
          <ul className="space-y-1 text-xs">
            {v.coverage.quality_flags.map((f, i) => (
              <li key={i}>
                <Badge variant="outline" className="mr-2">
                  {f.severity}
                </Badge>
                <span className="font-mono">{f.id}</span>
                {f.detail && (
                  <span className="text-muted-foreground"> — {f.detail}</span>
                )}
              </li>
            ))}
          </ul>
        </section>
      )}

      {v.ai_explanation && (
        <section>
          <h2 className="mb-2 text-sm font-semibold">AI explanation</h2>
          <blockquote className="border-l-2 pl-3 text-sm text-muted-foreground">
            {v.ai_explanation}
          </blockquote>
        </section>
      )}

      {/* Print-only footer (S4 will tighten). */}
      <footer className="hidden print:block text-[10px] text-muted-foreground">
        flow-agent insights mock-up · fixture-backed · not a production
        record
      </footer>
    </article>
  )
}

function Field({
  label,
  value,
  mono,
}: {
  label: string
  value: string
  mono?: boolean
}) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-[11px] uppercase tracking-wide text-muted-foreground">
        {label}
      </span>
      <span className={mono ? "font-mono text-xs" : "text-sm"}>{value}</span>
    </div>
  )
}

function renderEvidenceTable(rows: Array<Record<string, unknown>>) {
  const first = rows[0]
  if (!first) {
    return <p className="text-xs text-muted-foreground">No evidence rows.</p>
  }
  const keys = Object.keys(first)
  return (
    <div className="overflow-x-auto rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            {keys.map((k) => (
              <TableHead key={k}>{k}</TableHead>
            ))}
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row, i) => (
            <TableRow key={i}>
              {keys.map((k) => (
                <TableCell key={k} className="font-mono text-xs">
                  {formatCell(row[k])}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

function formatCell(v: unknown): string {
  if (v === null || v === undefined) return "—"
  if (typeof v === "object") return JSON.stringify(v)
  return String(v)
}
