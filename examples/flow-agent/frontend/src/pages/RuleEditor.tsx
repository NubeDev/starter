import { useEffect, useMemo, useState } from "react"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { useParams, Link } from "react-router-dom"
import { IconBulb, IconArrowLeft, IconPlayerPlay } from "@tabler/icons-react"

import { PageHero } from "@/components/page-hero"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { Input } from "@/components/ui/input"
import {
  Tabs,
  TabsList,
  TabsTrigger,
  TabsContent,
} from "@/components/ui/tabs"
import { Card, CardContent } from "@/components/ui/card"
import { api, type InsightsRule, type InsightsVerdict } from "@/lib/api"

export function RuleEditor() {
  const { id = "" } = useParams<{ id: string }>()
  const qc = useQueryClient()

  const ruleQ = useQuery({
    queryKey: ["insights", "rule", id],
    queryFn: () => api.insights.getRule(id),
    enabled: !!id,
  })

  // Local working copy. Reset whenever the underlying rule changes.
  const [body, setBody] = useState("")
  const [summary, setSummary] = useState("")
  const [tags, setTags] = useState("")
  const [dirty, setDirty] = useState(false)
  const [dryRun, setDryRun] = useState<InsightsVerdict | null>(null)

  useEffect(() => {
    if (!ruleQ.data) return
    setBody(ruleQ.data.body)
    setSummary(ruleQ.data.summary)
    setTags(ruleQ.data.tags.join(", "))
    setDirty(false)
  }, [ruleQ.data])

  const patchM = useMutation({
    mutationFn: (patch: Partial<InsightsRule>) =>
      api.insights.updateRule(id, patch),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["insights", "rule", id] })
      qc.invalidateQueries({ queryKey: ["insights", "rules"] })
      setDirty(false)
    },
  })

  const dryM = useMutation({
    mutationFn: () => api.insights.dryRunRule(id, { body }),
    onSuccess: (v) => setDryRun(v),
  })

  const onSave = () => {
    if (!ruleQ.data) return
    patchM.mutate({
      body,
      summary,
      tags: tags
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean),
    })
  }

  if (ruleQ.isLoading) {
    return <div className="px-4 py-6 lg:px-6">Loading…</div>
  }
  if (ruleQ.error || !ruleQ.data) {
    return (
      <div className="flex flex-col gap-2 px-4 py-6 lg:px-6">
        <p className="text-sm text-destructive">Rule not found.</p>
        <Link to="/insights/rules" className="text-sm underline">
          Back to rules
        </Link>
      </div>
    )
  }
  const r = ruleQ.data

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <div className="flex items-center justify-between">
        <Button variant="ghost" size="sm" asChild>
          <Link to="/insights/rules">
            <IconArrowLeft className="mr-1 size-4" />
            Back
          </Link>
        </Button>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => dryM.mutate()}
            disabled={dryM.isPending}
          >
            <IconPlayerPlay className="mr-1 size-4" />
            {dryM.isPending ? "Running…" : "Dry-run"}
          </Button>
          <Button size="sm" onClick={onSave} disabled={!dirty || patchM.isPending}>
            {patchM.isPending ? "Saving…" : dirty ? "Save changes" : "Saved"}
          </Button>
        </div>
      </div>

      <PageHero
        icon={IconBulb}
        accent="var(--accent-success)"
        title={r.id}
        description={r.summary}
        actions={
          <div className="flex items-center gap-2">
            <Badge variant="outline">{r.kind}</Badge>
            <Badge variant="secondary">{r.namespace}</Badge>
            <Badge>{r.severity_default}</Badge>
          </div>
        }
      />

      <div className="grid gap-4 lg:grid-cols-[2fr_1fr]">
        {/* Left: editor + schema */}
        <div className="flex flex-col gap-3">
          <Tabs defaultValue="body">
            <TabsList>
              <TabsTrigger value="body">Body</TabsTrigger>
              <TabsTrigger value="schema">Schema</TabsTrigger>
            </TabsList>
            <TabsContent value="body">
              <Textarea
                value={body}
                onChange={(e) => {
                  setBody(e.target.value)
                  setDirty(true)
                }}
                spellCheck={false}
                rows={18}
                className="font-mono text-xs"
              />
              <p className="mt-1 text-[10px] text-muted-foreground">
                Plain textarea for the mock-up. Monaco/CodeMirror deferred —
                see progress doc D-S5-1.
              </p>
            </TabsContent>
            <TabsContent value="schema">
              <SchemaPanel rule={r} />
            </TabsContent>
          </Tabs>

          <div className="grid gap-2 sm:grid-cols-2">
            <div>
              <label className="text-[11px] uppercase tracking-wide text-muted-foreground">
                Summary
              </label>
              <Input
                value={summary}
                onChange={(e) => {
                  setSummary(e.target.value)
                  setDirty(true)
                }}
              />
            </div>
            <div>
              <label className="text-[11px] uppercase tracking-wide text-muted-foreground">
                Tags (comma-separated)
              </label>
              <Input
                value={tags}
                onChange={(e) => {
                  setTags(e.target.value)
                  setDirty(true)
                }}
              />
            </div>
          </div>
        </div>

        {/* Right: dry-run result */}
        <DryRunPanel verdict={dryRun} pending={dryM.isPending} />
      </div>
    </div>
  )
}

function SchemaPanel({ rule }: { rule: InsightsRule }) {
  const rows = useMemo(() => {
    const out: Array<[string, string]> = [
      ["id", rule.id],
      ["kind", rule.kind],
      ["namespace", rule.namespace],
      ["severity_default", rule.severity_default],
      ["tags", rule.tags.join(", ")],
      ["created_at", rule.created_at],
      ["updated_at", rule.updated_at],
    ]
    for (const [k, v] of Object.entries(rule.schema)) {
      out.push([
        k,
        typeof v === "object" ? JSON.stringify(v) : String(v ?? "—"),
      ])
    }
    return out
  }, [rule])

  return (
    <Card>
      <CardContent className="grid gap-2 p-4 text-sm">
        {rows.map(([k, v]) => (
          <div key={k} className="grid grid-cols-[140px_1fr] gap-2">
            <span className="text-[11px] uppercase tracking-wide text-muted-foreground">
              {k}
            </span>
            <span className="font-mono text-xs break-all">{v || "—"}</span>
          </div>
        ))}
      </CardContent>
    </Card>
  )
}

function DryRunPanel({
  verdict,
  pending,
}: {
  verdict: InsightsVerdict | null
  pending: boolean
}) {
  return (
    <Card>
      <CardContent className="flex flex-col gap-3 p-4">
        <h2 className="text-sm font-semibold">Dry-run result</h2>
        {pending && <p className="text-xs text-muted-foreground">Running…</p>}
        {!pending && !verdict && (
          <p className="text-xs text-muted-foreground">
            Click <em>Dry-run</em> to synthesise a verdict from fixtures.
            No engine; the mock returns the most recent verdict for this
            rule with a <code>dry_run: true</code> marker.
          </p>
        )}
        {verdict && (
          <div className="flex flex-col gap-2 text-xs">
            <div className="flex items-center gap-2">
              <Badge>{verdict.severity}</Badge>
              <span className="text-muted-foreground">
                confidence{" "}
                {(verdict.coverage.effective.confidence * 100).toFixed(0)}%
              </span>
            </div>
            <p>{verdict.summary}</p>
            {verdict.evidence.length > 0 && (
              <pre className="overflow-x-auto rounded bg-muted p-2 text-[10px]">
                {JSON.stringify(verdict.evidence, null, 2)}
              </pre>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
