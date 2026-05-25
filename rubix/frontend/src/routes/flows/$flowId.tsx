// `/flows/$flowId` — live inspector + per-node settings editor for a
// single deployed flow.
//
// The route:
//   * Reads the deployed `body_yaml` from `useFlowsList()` (cached
//     under `['rubix','flow_ops','list']`) and parses it into the
//     `FlowGraph` the canvas wants. The list query is the source of
//     truth — `flowDeploy` invalidates the prefix on save so the
//     parsed graph re-flows automatically after a hot-reload.
//   * Subscribes to `useFlowEvents(flowId)` for the live SSE feed
//     and threads the aggregated `runOverlay` into `<FlowCanvas>`,
//     which in turn dispatches per-node `slotValues` badges via the
//     existing `useFlowGraph` wiring.
//   * Tracks the xyflow selection (single node id) via the
//     `reactFlowProps.onSelectionChange` escape hatch and forwards
//     it to `<SettingsSidebar>` so the operator can hot-edit the
//     selected node's config without leaving the page.

import { useMemo, useState } from 'react'
import { createFileRoute, Link, useParams } from '@tanstack/react-router'
import { useQueryClient } from '@tanstack/react-query'
import { useIntl } from 'react-intl'
import { ArrowLeft } from 'lucide-react'
import * as YAML from 'yaml'
import { FlowCanvas } from '@nube/starter-ui-flow'
import type { FlowGraph, RunOverlay } from '@nube/starter-ui-flow'
import { Button, Skeleton } from '@nube/starter-ui-kit'
import {
  FLOW_OPS_KEY,
  useFlowEvents,
  useFlowsList,
} from '@nube/rubix-client-react'
import type { FlowListResponse } from '@nube/rubix-client-ts'
import { buildFlowRegistry } from '@/lib/flow-registry'
import { ErrorBoundary } from '@/components/error-boundary'
import { SettingsSidebar } from './settings-sidebar'

// Build once at module load — the registry is immutable.
const flowRegistry = buildFlowRegistry()

/** Cache key the list query writes to; mirrored from `flow-ops.ts`. */
const FLOW_LIST_KEY = [...FLOW_OPS_KEY, 'list'] as const

/** Surface shape we care about when parsing `body_yaml`. */
interface ParsedFlow {
  nodes?: Array<{ id: string; kind: string; config?: unknown; label?: string }>
  links?: Array<{ from: string; to: string }>
}

/** Turn `RubixFlowYaml` into the `FlowGraph` the canvas renders. */
function yamlToGraph(bodyYaml: string): FlowGraph {
  let parsed: ParsedFlow = {}
  try {
    parsed = (YAML.parse(bodyYaml) as ParsedFlow) ?? {}
  } catch {
    return { nodes: [], edges: [] }
  }
  const nodes: FlowGraph['nodes'] = (parsed.nodes ?? []).map((n, i) => ({
    id: n.id,
    kind: n.kind,
    // Auto-layout: column-major because the bundled flows are tiny
    // linear pipelines today. A nicer dagre layout can land later.
    position: { x: 80 + i * 280, y: 160 },
    label: n.label ?? n.id,
    data: (n.config as Record<string, unknown> | undefined) ?? {},
  }))
  const edges: FlowGraph['edges'] = (parsed.links ?? []).map((l, i) => {
    const [src, srcSlot = 'out'] = l.from.split('.')
    const [tgt, tgtSlot = 'in'] = l.to.split('.')
    return {
      id: `e${i}-${src}-${tgt}`,
      source: src!,
      sourceSlot: srcSlot,
      target: tgt!,
      targetSlot: tgtSlot,
    }
  })
  return { nodes, edges }
}

function FlowDetail() {
  const { flowId } = useParams({ from: '/flows/$flowId' })
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const qc = useQueryClient()
  const list = useFlowsList()

  // Prefer the active query result; fall back to the cache lookup
  // the stage description specifies for the no-Provider edge case.
  const cached = qc.getQueryData<FlowListResponse>(FLOW_LIST_KEY)
  const item = useMemo(
    () =>
      (list.data?.flows ?? cached?.flows ?? []).find((f) => f.flow_id === flowId),
    [list.data, cached, flowId],
  )

  const bodyYaml = item?.body_yaml ?? ''
  const graph = useMemo(() => yamlToGraph(bodyYaml), [bodyYaml])

  const { runOverlay } = useFlowEvents(flowId)
  // Structural mirror → canvas `RunOverlay`; cast at the mount site
  // per the `useFlowEvents` docstring.
  const overlay = runOverlay as unknown as RunOverlay

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)

  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-6 flex items-end justify-between gap-4">
        <div>
          <div className="flex items-center gap-3">
            <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
              {tr('flows.detail.eyebrow', 'Flow')}
            </span>
          </div>
          <h1 className="mt-3 text-3xl font-medium tracking-[-0.03em]">{flowId}</h1>
          {item?.revision_id ? (
            <p className="mt-2 font-mono text-xs text-[color:var(--color-muted)]">
              {item.revision_id}
            </p>
          ) : null}
        </div>
        <Button asChild variant="ghost" size="sm">
          <Link to="/flows">
            <ArrowLeft className="mr-2 h-4 w-4" />
            {tr('flows.detail.back', 'Back to flows')}
          </Link>
        </Button>
      </header>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[1fr_320px]">
        {list.isLoading ? (
          <Skeleton className="h-[640px] w-full" />
        ) : (
          <div className="h-[640px] overflow-hidden rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)]">
            <FlowCanvas
              registry={flowRegistry}
              graph={graph}
              overlay={overlay}
              readOnly={false}
              showMiniMap
              showControls
              showBackground
              reactFlowProps={{
                onSelectionChange: ({ nodes }) => {
                  setSelectedNodeId(nodes.length === 1 ? (nodes[0]?.id ?? null) : null)
                },
              }}
            />
          </div>
        )}

        <SettingsSidebar
          flowId={flowId}
          selectedNodeId={selectedNodeId}
          bodyYaml={bodyYaml}
        />
      </div>
    </section>
  )
}

function FlowDetailRoute() {
  return (
    <ErrorBoundary>
      <FlowDetail />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/flows/$flowId')({ component: FlowDetailRoute })
