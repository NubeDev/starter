import { useMemo, useState, useEffect, useCallback } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { motion } from 'motion/react'
import { useIntl } from 'react-intl'
import { Play, RotateCcw, X } from 'lucide-react'

import {
  FlowCanvas,
  NodeKindRegistry,
  BUILTIN_NODE_KINDS,
  type FlowGraph,
  type FlowNode,
  type RunOverlay,
  type NodeRunState,
  type SlotName,
  type NodeId,
  type FlowMessages,
} from '@nube/starter-ui-flow'
import { Button } from '@/components/ui/button'

// ---------------------------------------------------------------------------
// Demo graph — matches the shape the engine will actually return.
// ---------------------------------------------------------------------------
// - `FlowNode.data` is the per-kind settings bag (typed against the
//   kind's JSON Schema from `NodeBehavior::config_schema()` server-side).
// - `RunOverlay.slotValues[node][slot]` mirrors `FlowEvent::NodeEmitted`
//   from `starter-flow-spi` — outputs only. Input badges are derived in
//   the UI from upstream edges, the same way the engine wires them.
// ---------------------------------------------------------------------------

const INITIAL_GRAPH: FlowGraph = {
  nodes: [
    {
      id: 'trigger-1',
      kind: 'trigger',
      position: { x: 0, y: 140 },
      label: 'Webhook',
      data: { event: 'http.request', path: '/inbox' },
    },
    {
      id: 'agent-1',
      kind: 'ai-agent',
      position: { x: 300, y: 100 },
      label: 'Classify intent',
      data: {
        model: 'gpt-4o-mini',
        system_prompt: 'You triage support requests into {urgent, normal, spam}.',
        temperature: 0.2,
        max_tokens: 256,
      },
    },
    {
      id: 'tool-1',
      kind: 'tool-call',
      position: { x: 620, y: 20 },
      label: 'Send Slack message',
      data: { tool: 'slack.post_message', channel: '#triage', mention: '@oncall' },
    },
    {
      id: 'branch-1',
      kind: 'branch',
      position: { x: 620, y: 200 },
      label: 'Urgent?',
      data: { expression: 'intent == "urgent"' },
    },
    {
      id: 'transform-1',
      kind: 'transform',
      position: { x: 940, y: 140 },
      label: 'Format payload',
      data: { script: 'omit(["raw_html","headers"]) | rename({"subject":"title"})' },
    },
    {
      id: 'tool-2',
      kind: 'tool-call',
      position: { x: 940, y: 280 },
      label: 'Open ticket',
      data: { tool: 'github.create_issue', repo: 'nube/ops', labels: ['triage'] },
    },
  ],
  edges: [
    { id: 'e1', source: 'trigger-1', sourceSlot: 'fire', target: 'agent-1',     targetSlot: 'in'   },
    { id: 'e2', source: 'agent-1',   sourceSlot: 'out',  target: 'tool-1',      targetSlot: 'args' },
    { id: 'e3', source: 'agent-1',   sourceSlot: 'out',  target: 'branch-1',    targetSlot: 'in'   },
    { id: 'e4', source: 'branch-1',  sourceSlot: 'then', target: 'transform-1', targetSlot: 'in'   },
    { id: 'e5', source: 'branch-1',  sourceSlot: 'else', target: 'tool-2',      targetSlot: 'args' },
  ],
}

// What the engine would emit per node (NodeEmitted.value, keyed by
// node id then output slot). The run simulator below reveals these
// incrementally — the same way real events stream in.
const FAKE_EMISSIONS: Record<NodeId, Record<SlotName, unknown>> = {
  'trigger-1':   { fire:   { ts: '2026-05-24T16:42:11Z', id: 'evt_8K3' } },
  'agent-1':     { out:    { intent: 'urgent', confidence: 0.93 }, events: '...streaming...' },
  'tool-1':      { result: { ok: true, channel: '#triage', message_ts: '1716568...' } },
  'branch-1':    { then:   true, else: false },
  'transform-1': { out:    { title: 'Lights out in EU-west', priority: 'P1' } },
  'tool-2':      { result: { issue_id: 4821, url: 'https://gh/nube/ops/issues/4821' } },
}

const RUN_SEQUENCE: Array<{ node: NodeId; edge?: string }> = [
  { node: 'trigger-1' },
  { node: 'agent-1',     edge: 'e1' },
  { node: 'branch-1',    edge: 'e3' },
  { node: 'tool-1',      edge: 'e2' },
  { node: 'transform-1', edge: 'e4' },
  { node: 'tool-2',      edge: 'e5' },
]

/**
 * Project engine output values onto input slots by walking edges.
 * The engine does this server-side via the `GraphStore::write_slot`
 * chokepoint; we mirror it here so the UI badges line up.
 */
function deriveInputValues(
  graph: FlowGraph,
  outputs: Record<NodeId, Record<SlotName, unknown>>,
): Record<NodeId, Record<SlotName, unknown>> {
  const merged: Record<NodeId, Record<SlotName, unknown>> = {}
  // Copy outputs through unchanged.
  for (const [n, m] of Object.entries(outputs)) merged[n] = { ...m }
  // Fan in: each edge carries the source's emitted value to the target slot.
  for (const e of graph.edges) {
    const srcVal = outputs[e.source]?.[e.sourceSlot]
    if (srcVal === undefined) continue
    merged[e.target] ??= {}
    merged[e.target]![e.targetSlot] = srcVal
  }
  return merged
}

function FlowPage() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const registry = useMemo(() => new NodeKindRegistry().registerAll(BUILTIN_NODE_KINDS), [])

  // Translations passed to the canvas. Built-in kinds get their
  // labels swapped via `kindLabels`; state-dot a11y text via `state`.
  const flowMessages = useMemo<Partial<FlowMessages>>(
    () => ({
      state: {
        idle: tr('flow.legend.idle'),
        ready: tr('flow.state.ready'),
        running: tr('flow.legend.running'),
        ok: tr('flow.legend.ok'),
        error: tr('flow.legend.error'),
        cancelled: tr('flow.state.cancelled'),
        skipped: tr('flow.state.skipped'),
      },
      kindLabels: {
        'ai-agent':  tr('flow.kind.ai-agent'),
        'tool-call': tr('flow.kind.tool-call'),
        'trigger':   tr('flow.kind.trigger'),
        'branch':    tr('flow.kind.branch'),
        'transform': tr('flow.kind.transform'),
        'subflow':   tr('flow.kind.subflow'),
      },
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [intl.locale],
  )

  const [graph, setGraph] = useState<FlowGraph>(INITIAL_GRAPH)
  const [overlay, setOverlay] = useState<RunOverlay | undefined>(undefined)
  const [runId, setRunId] = useState(0)
  const [selectedId, setSelectedId] = useState<NodeId | null>(null)

  // Walk RUN_SEQUENCE, marking nodes running → ok and revealing each
  // node's emitted outputs as it completes. Input badges flow forward
  // automatically via deriveInputValues below.
  useEffect(() => {
    if (runId === 0) return
    let cancelled = false
    const nodes: Record<string, NodeRunState> = {}
    const outputs: Record<NodeId, Record<SlotName, unknown>> = {}
    let activeEdges: string[] = []
    let i = 0
    const tick = () => {
      if (cancelled) return
      if (i >= RUN_SEQUENCE.length) {
        setOverlay({
          nodes: { ...nodes },
          activeEdges: [],
          slotValues: deriveInputValues(graph, outputs),
        })
        return
      }
      const step = RUN_SEQUENCE[i]!
      // Promote prior running → ok and reveal its outputs.
      for (const id of Object.keys(nodes)) {
        if (nodes[id] === 'running') {
          nodes[id] = 'ok'
          if (FAKE_EMISSIONS[id]) outputs[id] = FAKE_EMISSIONS[id]
        }
      }
      nodes[step.node] = 'running'
      activeEdges = step.edge ? [step.edge] : []
      setOverlay({
        nodes: { ...nodes },
        activeEdges: [...activeEdges],
        slotValues: deriveInputValues(graph, outputs),
      })
      i++
      setTimeout(tick, 850)
    }
    tick()
    return () => {
      cancelled = true
    }
  }, [runId, graph])

  const reset = useCallback(() => {
    setGraph(INITIAL_GRAPH)
    setOverlay(undefined)
    setRunId(0)
    setSelectedId(null)
  }, [])

  const selectedNode = selectedId ? graph.nodes.find((n) => n.id === selectedId) ?? null : null

  // Persist a settings edit from the side panel back into the graph.
  const updateNodeData = useCallback(
    (id: NodeId, next: Record<string, unknown>) => {
      setGraph((g) => ({
        ...g,
        nodes: g.nodes.map((n) => (n.id === id ? { ...n, data: next } : n)),
      }))
    },
    [],
  )

  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <motion.div
        initial={{ opacity: 0, y: 14 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
        className="mb-8 flex items-end justify-between gap-4"
      >
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-3">
            <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
            <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
              {tr('flow.eyebrow')}
            </span>
          </div>
          <h1 className="max-w-3xl text-4xl font-medium leading-[1.05] tracking-[-0.03em] text-[color:var(--color-text)] sm:text-5xl">
            {tr('flow.titlePrefix')}{' '}
            <span className="serif-italic text-[color:var(--color-leaf)]">{tr('flow.titleAccent')}</span>
          </h1>
          <p className="max-w-2xl text-sm text-[color:var(--color-subtle)]">{tr('flow.subtitle')}</p>
        </div>
        <div className="flex shrink-0 gap-2">
          <Button variant="outline" size="sm" onClick={reset}>
            <RotateCcw className="mr-2 h-3.5 w-3.5" />
            {tr('flow.toolbar.reset')}
          </Button>
          <Button size="sm" onClick={() => setRunId((n) => n + 1)}>
            <Play className="mr-2 h-3.5 w-3.5" />
            {tr('flow.toolbar.run')}
          </Button>
        </div>
      </motion.div>

      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.7, delay: 0.1, ease: [0.22, 1, 0.36, 1] }}
        className="glass hairline relative h-[640px] overflow-hidden rounded-3xl p-1"
      >
        <FlowCanvas
          registry={registry}
          graph={graph}
          overlay={overlay}
          onChange={setGraph}
          i18n={flowMessages}
          className="h-full w-full overflow-hidden rounded-[22px]"
          reactFlowProps={{
            onNodeClick: (_e, node) => setSelectedId(node.id),
            onPaneClick: () => setSelectedId(null),
          }}
        />
        <Legend />
        {selectedNode ? (
          <NodeInspector
            node={selectedNode}
            slotValues={overlay?.slotValues?.[selectedNode.id]}
            onClose={() => setSelectedId(null)}
            onChange={(next) => updateNodeData(selectedNode.id, next)}
          />
        ) : null}
      </motion.div>
    </section>
  )
}

function Legend() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const dots: Array<{ id: string; color: string }> = [
    { id: 'flow.legend.idle',    color: 'var(--color-border)' },
    { id: 'flow.legend.running', color: 'var(--color-sun)' },
    { id: 'flow.legend.ok',      color: 'var(--color-leaf)' },
    { id: 'flow.legend.error',   color: 'var(--color-danger)' },
  ]
  return (
    <div className="pointer-events-none absolute bottom-6 left-6 z-10 flex items-center gap-4 rounded-full border border-[color:var(--color-border)] bg-[color:var(--color-surface)]/85 px-4 py-2 text-[11px] backdrop-blur-md">
      <span className="font-medium uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
        {tr('flow.legend.title')}
      </span>
      {dots.map((d) => (
        <span key={d.id} className="flex items-center gap-1.5 text-[color:var(--color-text)]">
          <span
            className="h-2 w-2 rounded-full"
            style={{ background: d.color, boxShadow: `0 0 0 2px color-mix(in oklab, ${d.color} 25%, transparent)` }}
          />
          {tr(d.id)}
        </span>
      ))}
    </div>
  )
}

// ---------------------------------------------------------------------------
// Side panel — settings editor + live slot values.
// ---------------------------------------------------------------------------

function NodeInspector({
  node,
  slotValues,
  onClose,
  onChange,
}: {
  node: FlowNode
  slotValues: Record<SlotName, unknown> | undefined
  onClose: () => void
  onChange: (next: Record<string, unknown>) => void
}) {
  const intl = useIntl()
  return (
    <motion.aside
      initial={{ opacity: 0, x: 24 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: 24 }}
      transition={{ duration: 0.25, ease: [0.22, 1, 0.36, 1] }}
      className="absolute right-3 top-3 bottom-3 z-20 flex w-[360px] flex-col gap-4 overflow-hidden rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface)]/95 p-4 shadow-[0_18px_46px_-22px_rgba(15,23,42,0.45)] backdrop-blur-xl"
    >
      <header className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            {node.kind}
          </p>
          <h2 className="truncate text-sm font-semibold tracking-tight text-[color:var(--color-text)]">
            {node.label ?? node.kind}
          </h2>
          <p className="mt-0.5 truncate font-mono text-[10px] text-[color:var(--color-subtle)]">
            {node.id}
          </p>
        </div>
        <button
          onClick={onClose}
          className="-mt-1 -mr-1 rounded-md p-1 text-[color:var(--color-subtle)] hover:bg-[color:var(--color-surface-2)] hover:text-[color:var(--color-text)]"
          aria-label={intl.formatMessage({ id: 'flow.inspector.close' })}
        >
          <X className="h-4 w-4" />
        </button>
      </header>

      <SettingsEditor data={node.data ?? {}} onChange={onChange} />

      <SlotValuesPanel values={slotValues} />
    </motion.aside>
  )
}

function SettingsEditor({
  data,
  onChange,
}: {
  data: Record<string, unknown>
  onChange: (next: Record<string, unknown>) => void
}) {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const [draft, setDraft] = useState(() => JSON.stringify(data, null, 2))
  const [error, setError] = useState<string | null>(null)

  // Re-sync when external data changes (e.g. reset).
  useEffect(() => {
    setDraft(JSON.stringify(data, null, 2))
    setError(null)
  }, [data])

  const commit = useCallback(() => {
    try {
      const next = JSON.parse(draft)
      if (next === null || typeof next !== 'object' || Array.isArray(next)) {
        setError(tr('flow.inspector.settingsInvalid'))
        return
      }
      setError(null)
      onChange(next as Record<string, unknown>)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draft, onChange, intl.locale])

  return (
    <section className="flex min-h-0 flex-col gap-2">
      <div className="flex items-center justify-between">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.16em] text-[color:var(--color-subtle)]">
          {tr('flow.inspector.settings')}
        </h3>
        <span className="text-[10px] text-[color:var(--color-subtle)]">node.data</span>
      </div>
      <textarea
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        spellCheck={false}
        className="h-44 w-full resize-none rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-bg)] p-2 font-mono text-[11px] leading-relaxed text-[color:var(--color-text)] outline-none focus:border-[color:var(--color-leaf)] focus:ring-1 focus:ring-[color:var(--color-leaf)]/40"
      />
      {error ? (
        <p className="text-[11px] text-[color:var(--color-danger)]">{error}</p>
      ) : (
        <p className="text-[10px] text-[color:var(--color-subtle)]">
          {tr('flow.inspector.settingsHint')}
        </p>
      )}
    </section>
  )
}

function SlotValuesPanel({ values }: { values: Record<SlotName, unknown> | undefined }) {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const entries = values ? Object.entries(values) : []
  return (
    <section className="flex min-h-0 flex-1 flex-col gap-2">
      <div className="flex items-center justify-between">
        <h3 className="text-[11px] font-semibold uppercase tracking-[0.16em] text-[color:var(--color-subtle)]">
          {tr('flow.inspector.slotValues')}
        </h3>
        <span className="text-[10px] text-[color:var(--color-subtle)]">{tr('flow.inspector.live')}</span>
      </div>
      {entries.length === 0 ? (
        <p className="rounded-md border border-dashed border-[color:var(--color-border)] p-3 text-[11px] text-[color:var(--color-subtle)]">
          {intl.formatMessage(
            { id: 'flow.inspector.empty' },
            {
              action: (
                <span className="font-medium text-[color:var(--color-text)]">
                  {tr('flow.toolbar.run')}
                </span>
              ),
            },
          )}
        </p>
      ) : (
        <ul className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          {entries.map(([slot, value]) => (
            <li
              key={slot}
              className="flex flex-col gap-1 rounded-md border border-[color:var(--color-border)] bg-[color:var(--color-bg)] p-2"
            >
              <span className="font-mono text-[10px] uppercase tracking-wider text-[color:var(--color-subtle)]">
                {slot}
              </span>
              <code className="overflow-x-auto whitespace-pre-wrap break-all font-mono text-[11px] text-[color:var(--color-text)]">
                {formatValue(value)}
              </code>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

function formatValue(v: unknown): string {
  if (v === null || v === undefined) return String(v)
  if (typeof v === 'string') return v
  if (typeof v === 'number' || typeof v === 'boolean' || typeof v === 'bigint') return String(v)
  try {
    return JSON.stringify(v, null, 2)
  } catch {
    return String(v)
  }
}

export const Route = createFileRoute('/flow')({ component: FlowPage })
