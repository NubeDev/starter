import { useMemo, useState, useEffect } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { motion } from 'motion/react'
import { useIntl } from 'react-intl'
import { Play, Shuffle, RotateCcw } from 'lucide-react'

import {
  FlowCanvas,
  NodeKindRegistry,
  BUILTIN_NODE_KINDS,
  type FlowGraph,
  type RunOverlay,
  type NodeRunState,
} from '@nube/starter-ui-flow'
import { Button } from '@/components/ui/button'

// ---------------------------------------------------------------------------
// Demo graph
// ---------------------------------------------------------------------------
// Synthetic but realistic: a webhook trigger wakes an AI agent, which
// either calls a tool or branches to a transform. No backend; data
// lives in component state so users can drag, connect, and reset.
// ---------------------------------------------------------------------------

const INITIAL_GRAPH: FlowGraph = {
  nodes: [
    { id: 'trigger-1',   kind: 'trigger',   position: { x:   0, y: 140 }, label: 'Webhook' },
    { id: 'agent-1',     kind: 'ai-agent',  position: { x: 280, y: 100 }, label: 'Classify intent' },
    { id: 'tool-1',      kind: 'tool-call', position: { x: 580, y:  20 }, label: 'Send Slack message' },
    { id: 'branch-1',    kind: 'branch',    position: { x: 580, y: 200 }, label: 'Urgent?' },
    { id: 'transform-1', kind: 'transform', position: { x: 880, y: 140 }, label: 'Format payload' },
    { id: 'tool-2',      kind: 'tool-call', position: { x: 880, y: 280 }, label: 'Open ticket' },
  ],
  edges: [
    { id: 'e1', source: 'trigger-1', sourceSlot: 'fire', target: 'agent-1',     targetSlot: 'in'   },
    { id: 'e2', source: 'agent-1',   sourceSlot: 'out',  target: 'tool-1',      targetSlot: 'args' },
    { id: 'e3', source: 'agent-1',   sourceSlot: 'out',  target: 'branch-1',    targetSlot: 'in'   },
    { id: 'e4', source: 'branch-1',  sourceSlot: 'then', target: 'transform-1', targetSlot: 'in'   },
    { id: 'e5', source: 'branch-1',  sourceSlot: 'else', target: 'tool-2',      targetSlot: 'args' },
  ],
}

const RUN_SEQUENCE: Array<{ node: string; edge?: string }> = [
  { node: 'trigger-1' },
  { node: 'agent-1',     edge: 'e1' },
  { node: 'branch-1',    edge: 'e3' },
  { node: 'tool-1',      edge: 'e2' },
  { node: 'transform-1', edge: 'e4' },
  { node: 'tool-2',      edge: 'e5' },
]

function FlowPage() {
  const intl = useIntl()
  const tr = (id: string) => intl.formatMessage({ id })
  const registry = useMemo(() => new NodeKindRegistry().registerAll(BUILTIN_NODE_KINDS), [])

  const [graph, setGraph] = useState<FlowGraph>(INITIAL_GRAPH)
  const [overlay, setOverlay] = useState<RunOverlay | undefined>(undefined)
  const [runId, setRunId] = useState(0)

  // Simulated run: walk RUN_SEQUENCE marking nodes running → ok with
  // active edges between. Cancels on unmount or when reset fires.
  useEffect(() => {
    if (runId === 0) return
    let cancelled = false
    const nodes: Record<string, NodeRunState> = {}
    let activeEdges: string[] = []
    let i = 0
    const tick = () => {
      if (cancelled) return
      if (i >= RUN_SEQUENCE.length) {
        setOverlay({ nodes: { ...nodes }, activeEdges: [] })
        return
      }
      const step = RUN_SEQUENCE[i]!
      for (const id of Object.keys(nodes)) {
        if (nodes[id] === 'running') nodes[id] = 'ok'
      }
      nodes[step.node] = 'running'
      activeEdges = step.edge ? [step.edge] : []
      setOverlay({ nodes: { ...nodes }, activeEdges: [...activeEdges] })
      i++
      setTimeout(tick, 750)
    }
    tick()
    return () => {
      cancelled = true
    }
  }, [runId])

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
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setGraph(INITIAL_GRAPH)
              setOverlay(undefined)
              setRunId(0)
            }}
          >
            <RotateCcw className="mr-2 h-3.5 w-3.5" />
            {tr('flow.toolbar.reset')}
          </Button>
          <Button variant="outline" size="sm" onClick={() => setGraph(INITIAL_GRAPH)}>
            <Shuffle className="mr-2 h-3.5 w-3.5" />
            {tr('flow.toolbar.layout')}
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
          className="h-full w-full overflow-hidden rounded-[22px]"
        />
        <Legend />
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

export const Route = createFileRoute('/flow')({ component: FlowPage })
