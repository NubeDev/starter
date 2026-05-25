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
import {
  Button,
  PageContainer,
  PageHeader,
  Skeleton,
} from '@nube/starter-ui-kit'
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
    // The flow editor benefits from every pixel of horizontal space, so we
    // opt out of the standard `max-w-7xl` shell and let the canvas span the
    // full viewport. The vertical `calc(100vh - …)` accounts for the fixed
    // top header (~6rem) plus this page's own header + padding (~7rem).
    <PageContainer width="full">
      <PageHeader
        eyebrow={tr('flows.detail.eyebrow', 'Flow')}
        title={flowId}
        description={
          def.data?.revision_id ? (
            <span className="font-mono text-xs">{def.data.revision_id}</span>
          ) : undefined
        }
        actions={
          <Button asChild variant="ghost" size="sm">
            <Link to="/flows">
              <ArrowLeft className="mr-2 h-4 w-4" />
              {tr('flows.detail.back', 'Back to flows')}
            </Link>
          </Button>
        }
      />

      {def.data?.placeholder ? (
        <div className="mb-4 rounded-xl border border-[color:var(--color-sun)]/40 bg-[color:var(--color-sun)]/10 px-4 py-3 text-sm text-[color:var(--color-sun)]">
          {tr(
            'flows.detail.placeholder',
            'Placeholder graph — rubix-agent does not yet expose a flow-body endpoint, so the canvas shows a stub node instead of the deployed YAML.',
          )}
        </div>
      ) : null}

      {def.isLoading ? (
        <Skeleton className="h-[calc(100vh-13rem)] w-full" />
      ) : (
        <div className="h-[calc(100vh-13rem)] min-h-[480px] overflow-hidden rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)]">
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
    </PageContainer>
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
