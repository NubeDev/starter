// `/flows/$flowId` — read-only inspector for a single deployed flow.
//
// Reads `useFlowDefinition(flowId)` and renders the resulting
// `FlowGraph` in `<FlowCanvas readOnly>`, using the rubix-side
// `flowRegistry` built once at module load (see
// `src/lib/flow-registry.ts`).
//
// While rubix-agent lacks a body-returning endpoint the hook
// synthesises a placeholder graph from the list metadata. A small
// banner surfaces that fact so an operator isn't misled into
// thinking the canvas reflects the deployed YAML.

import { useMemo } from 'react'
import { createFileRoute, Link, useParams } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import { ArrowLeft } from 'lucide-react'
import { FlowCanvas } from '@nube/starter-ui-flow'
import type { FlowGraph } from '@nube/starter-ui-flow'
import { Button, Skeleton } from '@nube/starter-ui-kit'
import { useFlowDefinition } from '@nube/rubix-client-react'
import { buildFlowRegistry } from '@/lib/flow-registry'
import { ErrorBoundary } from '@/components/error-boundary'

// Build once at module load — the registry is immutable.
const flowRegistry = buildFlowRegistry()

function FlowDetail() {
  const { flowId } = useParams({ from: '/flows/$flowId' })
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  const def = useFlowDefinition(flowId)
  const flowGraph = useMemo<FlowGraph>(
    () => (def.data?.graph as FlowGraph | undefined) ?? { nodes: [], edges: [] },
    [def.data],
  )

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
          {def.data?.revision_id ? (
            <p className="mt-2 font-mono text-xs text-[color:var(--color-muted)]">
              {def.data.revision_id}
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

      {def.data?.placeholder ? (
        <div className="mb-4 rounded-xl border border-[color:var(--color-sun)]/40 bg-[color:var(--color-sun)]/10 px-4 py-3 text-sm text-[color:var(--color-sun)]">
          {tr(
            'flows.detail.placeholder',
            'Placeholder graph — rubix-agent does not yet expose a flow-body endpoint, so the canvas shows a stub node instead of the deployed YAML.',
          )}
        </div>
      ) : null}

      {def.isLoading ? (
        <Skeleton className="h-[640px] w-full" />
      ) : (
        <div className="h-[640px] overflow-hidden rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)]">
          <FlowCanvas
            registry={flowRegistry}
            graph={flowGraph}
            readOnly={true}
            showMiniMap
            showControls
            showBackground
          />
        </div>
      )}
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
