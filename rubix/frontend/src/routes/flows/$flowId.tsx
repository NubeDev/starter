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

import { useMemo, useRef, useState } from 'react'
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
  useFlowDeploy,
  useFlowEvents,
  useFlowKinds,
  useFlowsList,
} from '@nube/rubix-client-react'
import type { FlowListResponse } from '@nube/rubix-client-ts'
import { appendFlowNode, FLOW_KIND_DRAG_MIME } from '@/lib/append-flow-node'
import { buildFlowRegistry } from '@/lib/flow-registry'
import { syncFlowGraph } from '@/lib/sync-flow-graph'
import { ErrorBoundary } from '@/components/error-boundary'
import { NodePalette } from '@/components/node-palette'
import { SettingsSidebar } from './settings-sidebar'

// Build once at module load — the registry is immutable.
const flowRegistry = buildFlowRegistry()

/** Cache key the list query writes to; mirrored from `flow-ops.ts`. */
const FLOW_LIST_KEY = [...FLOW_OPS_KEY, 'list'] as const

/** Surface shape we care about when parsing `body_yaml`. */
interface ParsedFlow {
  nodes?: Array<{
    id: string
    kind: string
    config?: unknown
    label?: string
    position?: { x: number; y: number }
  }>
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
    // Persisted position wins; fall back to a column auto-layout
    // so freshly-bundled flows that never went through the editor
    // still render in a sensible row.
    position:
      n.position && typeof n.position.x === 'number' && typeof n.position.y === 'number'
        ? { x: n.position.x, y: n.position.y }
        : { x: 80 + i * 280, y: 160 },
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

  // Drag-drop wiring: palette items drop here. We don't translate
  // pointer coordinates into flow space (positions aren't persisted
  // in the YAML body — the `yamlToGraph` auto-layout decides where
  // each node lands), so the drop handler just appends to the YAML
  // and triggers a deploy. The list query re-fetches and the new
  // node materialises on the canvas.
  const kinds = useFlowKinds()
  const deploy = useFlowDeploy()
  const [dropError, setDropError] = useState<string | null>(null)
  const handleDrop = async (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault()
    const kindId =
      event.dataTransfer.getData(FLOW_KIND_DRAG_MIME) ||
      event.dataTransfer.getData('text/plain')
    if (!kindId) return
    const kind = kinds.data?.kinds.find((k) => k.kind_id === kindId)
    if (!kind) {
      setDropError(`unknown kind ${kindId}`)
      return
    }
    if (!bodyYaml) {
      setDropError('flow body not loaded yet')
      return
    }
    setDropError(null)
    try {
      const nextYaml = appendFlowNode(bodyYaml, kind)
      await deploy.mutateAsync({ flow_id: flowId, body_yaml: nextYaml })
    } catch (err) {
      setDropError(err instanceof Error ? err.message : String(err))
    }
  }

  // Persist canvas-side structural mutations (node delete, new edge
  // via slot-handle drag) back to the YAML body and deploy. xyflow
  // fires `onChange` on every node move too; `syncFlowGraph` returns
  // `null` for those (position-only) cases so we skip the deploy.
  //
  // A microtask debounce coalesces the rapid-fire pair xyflow emits
  // when a node deletion cascades into edge removal — without it we
  // fire two deploys, the first carrying a stale `{ nodes still
  // present, edges already pruned }` body that the backend happily
  // accepts as the final state.
  const pendingChangeRef = useRef<FlowGraph | null>(null)
  const handleCanvasChange = (next: FlowGraph) => {
    if (!bodyYaml) return
    const wasPending = pendingChangeRef.current !== null
    pendingChangeRef.current = next
    if (wasPending) return
    queueMicrotask(() => {
      const latest = pendingChangeRef.current
      pendingChangeRef.current = null
      if (!latest || !bodyYaml) return
      const nextYaml = syncFlowGraph(bodyYaml, latest)
      if (!nextYaml) return
      setDropError(null)
      deploy.mutate(
        { flow_id: flowId, body_yaml: nextYaml },
        {
          onError: (err) => {
            setDropError(err instanceof Error ? err.message : String(err))
          },
        },
      )
    })
  }

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

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-[260px_1fr_320px]">
        <NodePalette flowId={flowId} bodyYaml={bodyYaml} />

        {list.isLoading ? (
          <Skeleton className="h-[640px] w-full" />
        ) : (
          <div
            className="h-[640px] overflow-hidden rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)]"
            onDragOver={(e) => {
              // Calling preventDefault on dragover is what makes the
              // element a valid drop target. Without it, `onDrop`
              // never fires.
              e.preventDefault()
              e.dataTransfer.dropEffect = 'copy'
            }}
            onDrop={handleDrop}
          >
            <FlowCanvas
              registry={flowRegistry}
              graph={graph}
              overlay={overlay}
              readOnly={false}
              showMiniMap
              showControls
              showBackground
              onChange={handleCanvasChange}
              reactFlowProps={{
                onSelectionChange: ({ nodes }) => {
                  setSelectedNodeId(nodes.length === 1 ? (nodes[0]?.id ?? null) : null)
                },
              }}
            />
            {dropError ? (
              <p className="px-3 py-1 text-xs text-red-500">{dropError}</p>
            ) : null}
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
