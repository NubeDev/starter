import { useMemo, useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { IconSitemap } from "@tabler/icons-react"

import { PageHero } from "@/components/page-hero"
import { Badge } from "@/components/ui/badge"
import { Card, CardContent } from "@/components/ui/card"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { api, type InsightsPipeline } from "@/lib/api"

export function PipelineCanvas() {
  const list = useQuery({
    queryKey: ["insights", "pipelines"],
    queryFn: api.insights.listPipelines,
  })
  const [selectedId, setSelectedId] = useState<string | null>(null)

  const pipelines = list.data ?? []
  const selected =
    pipelines.find((p) => p.id === selectedId) ?? pipelines[0] ?? null

  return (
    <div className="flex flex-col gap-6 px-4 py-6 lg:px-6">
      <PageHero
        icon={IconSitemap}
        accent="var(--accent-flows)"
        title="Insights · Pipelines"
        description="Fixture-backed pipeline graphs. Edits land via the agent (insights:pipeline.* tools)."
        actions={<Badge variant="secondary">{pipelines.length} total</Badge>}
      />

      {pipelines.length === 0 && !list.isLoading ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <IconSitemap />
            </EmptyMedia>
            <EmptyTitle>No pipelines yet</EmptyTitle>
            <EmptyDescription>
              Seed <code>fixtures/insights/pipelines.json</code>.
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="grid gap-4 lg:grid-cols-[260px_1fr]">
          <PipelineList
            pipelines={pipelines}
            selectedId={selected?.id ?? null}
            onSelect={(id) => setSelectedId(id)}
          />
          {selected && <PipelineGraph pipeline={selected} />}
        </div>
      )}
    </div>
  )
}

function PipelineList({
  pipelines,
  selectedId,
  onSelect,
}: {
  pipelines: InsightsPipeline[]
  selectedId: string | null
  onSelect: (id: string) => void
}) {
  return (
    <Card>
      <CardContent className="flex flex-col gap-1 p-2">
        {pipelines.map((p) => {
          const active = p.id === selectedId
          return (
            <button
              key={p.id}
              type="button"
              onClick={() => onSelect(p.id)}
              className={`flex flex-col gap-0.5 rounded-md px-3 py-2 text-left text-sm transition-colors ${
                active ? "bg-accent" : "hover:bg-accent/60"
              }`}
            >
              <span className="font-medium">{p.name}</span>
              <span className="font-mono text-[10px] text-muted-foreground">
                {p.id}
              </span>
              <div className="flex flex-wrap gap-1 pt-1">
                {p.tags.slice(0, 3).map((t) => (
                  <Badge
                    key={t}
                    variant="secondary"
                    className="text-[10px] font-normal"
                  >
                    {t}
                  </Badge>
                ))}
              </div>
            </button>
          )
        })}
      </CardContent>
    </Card>
  )
}

function PipelineGraph({ pipeline }: { pipeline: InsightsPipeline }) {
  const { width, height } = useMemo(() => {
    const nodes = pipeline.graph.nodes
    const maxX = nodes.reduce((m, n) => Math.max(m, n.x), 0) + 200
    const maxY = nodes.reduce((m, n) => Math.max(m, n.y), 0) + 120
    return { width: Math.max(maxX, 480), height: Math.max(maxY, 320) }
  }, [pipeline])

  const nodeIndex = useMemo(() => {
    const m = new Map<string, { x: number; y: number }>()
    pipeline.graph.nodes.forEach((n) => m.set(n.id, { x: n.x, y: n.y }))
    return m
  }, [pipeline])

  return (
    <Card>
      <CardContent className="p-4">
        <div className="mb-3 flex items-center justify-between">
          <div>
            <h2 className="text-sm font-semibold">{pipeline.name}</h2>
            {pipeline.description && (
              <p className="text-xs text-muted-foreground">
                {pipeline.description}
              </p>
            )}
          </div>
          <span className="text-[10px] text-muted-foreground">
            updated {new Date(pipeline.updated_at).toLocaleDateString()}
          </span>
        </div>

        <div className="overflow-auto rounded-md border bg-background">
          <svg
            width={width}
            height={height}
            viewBox={`0 0 ${width} ${height}`}
            role="img"
            aria-label={`Pipeline ${pipeline.name}`}
          >
            {/* edges */}
            {pipeline.graph.edges.map((e, i) => {
              const a = nodeIndex.get(e.from)
              const b = nodeIndex.get(e.to)
              if (!a || !b) return null
              const x1 = a.x + 160
              const y1 = a.y + 30
              const x2 = b.x
              const y2 = b.y + 30
              const mx = (x1 + x2) / 2
              return (
                <g key={i}>
                  <path
                    d={`M ${x1},${y1} C ${mx},${y1} ${mx},${y2} ${x2},${y2}`}
                    fill="none"
                    stroke={edgeColor(e.type)}
                    strokeWidth={1.5}
                    strokeDasharray={e.type === "Frame" ? "4 3" : undefined}
                  />
                  <text
                    x={mx}
                    y={(y1 + y2) / 2 - 4}
                    textAnchor="middle"
                    className="fill-muted-foreground"
                    fontSize={9}
                  >
                    {e.type}
                  </text>
                </g>
              )
            })}
            {/* nodes */}
            {pipeline.graph.nodes.map((n) => (
              <g key={n.id} transform={`translate(${n.x}, ${n.y})`}>
                <rect
                  width={160}
                  height={60}
                  rx={8}
                  ry={8}
                  className="fill-card stroke-border"
                  strokeWidth={1}
                />
                <text
                  x={10}
                  y={20}
                  className="fill-foreground"
                  fontSize={11}
                  fontWeight={600}
                >
                  {n.kind}
                </text>
                <text
                  x={10}
                  y={36}
                  className="fill-muted-foreground"
                  fontSize={10}
                >
                  {n.id}
                </text>
                {n.rule_id && (
                  <text
                    x={10}
                    y={50}
                    className="fill-muted-foreground"
                    fontFamily="monospace"
                    fontSize={9}
                  >
                    {n.rule_id}
                  </text>
                )}
              </g>
            ))}
          </svg>
        </div>

        <p className="mt-2 text-[10px] text-muted-foreground">
          Read-only viewer. Edits land via the agent (
          <code>insights:pipeline.propose-edit</code> →{" "}
          <code>insights:pipeline.apply-edit</code>).
        </p>
      </CardContent>
    </Card>
  )
}

function edgeColor(kind: string): string {
  switch (kind) {
    case "Verdict":
      return "var(--color-orange-500, oklch(0.72 0.18 60))"
    case "Dataset":
      return "var(--color-blue-500, oklch(0.65 0.18 240))"
    default:
      return "currentColor"
  }
}
