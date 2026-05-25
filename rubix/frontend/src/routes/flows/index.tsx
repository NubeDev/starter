// `/flows` — deployed-flow browser.
//
// Reads `useFlowsList()` from `@nube/rubix-client-react`. Each row is a
// link to `/flows/$flowId`. Loading state uses `<Skeleton>` and the
// empty state uses the `<Empty>` primitive — both from
// `@nube/starter-ui-kit`.
//
// `last_deployed_at` and `supersession_count` are part of the
// stage-6 column spec but rubix-agent's `rubix.flow_ops.list` reply
// only carries `{flow_id, revision_id}` today (see stage 3 BLOCKED
// handover for the missing-endpoint paragraph). Both columns render
// an em-dash placeholder so they appear in the header and the
// layout is ready once the backend grows the extra fields.

import { createFileRoute, Link } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import { Boxes, GitBranch } from 'lucide-react'
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  PageContainer,
  PageHeader,
  Skeleton,
} from '@nube/starter-ui-kit'
import { useFlowsList } from '@nube/rubix-client-react'
import { ErrorBoundary } from '@/components/error-boundary'

function FlowsTable() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })
  const list = useFlowsList()
  const rows = list.data?.flows ?? []

  return (
    <PageContainer width="wide">
      <PageHeader
        className="mb-8"
        eyebrow={tr('flows.eyebrow', 'Flows')}
        title={tr('flows.title', 'Deployed flows')}
        description={tr(
          'flows.subtitle',
          'Every flow with a live, non-superseded revision served by this rubix-agent. Click a row to inspect its graph.',
        )}
      />

      {list.isLoading ? (
        <div className="space-y-3">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : rows.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <Boxes />
            </EmptyMedia>
            <EmptyTitle>{tr('flows.empty.title', 'No deployed flows')}</EmptyTitle>
            <EmptyDescription>
              {tr(
                'flows.empty.body',
                'Use `rubix.flow_ops.deploy` to publish a revision and it will appear here.',
              )}
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="overflow-hidden rounded-2xl border border-[color:var(--color-border)] bg-[color:var(--color-surface-1)]">
          <table className="w-full text-left text-sm">
            <thead className="bg-[color:var(--color-surface-2)] text-[11px] uppercase tracking-[0.18em] text-[color:var(--color-muted)]">
              <tr>
                <th className="px-5 py-3 font-medium">
                  {tr('flows.col.flow_id', 'Flow id')}
                </th>
                <th className="px-5 py-3 font-medium">
                  {tr('flows.col.latest_revision_id', 'Latest revision')}
                </th>
                <th className="px-5 py-3 font-medium">
                  {tr('flows.col.last_deployed_at', 'Last deployed')}
                </th>
                <th className="px-5 py-3 font-medium">
                  {tr('flows.col.supersession_count', 'Supersessions')}
                </th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr
                  key={r.flow_id}
                  className="cursor-pointer border-t border-[color:var(--color-border)] transition hover:bg-[color:var(--color-surface-2)]/60"
                >
                  <td className="px-5 py-3 font-medium">
                    <Link
                      to="/flows/$flowId"
                      params={{ flowId: r.flow_id }}
                      className="flex items-center gap-2"
                    >
                      <GitBranch className="h-4 w-4 text-[color:var(--color-leaf)]" />
                      {r.flow_id}
                    </Link>
                  </td>
                  <td className="px-5 py-3 font-mono text-xs text-[color:var(--color-muted)]">
                    {r.revision_id.slice(0, 12)}
                  </td>
                  <td className="px-5 py-3 text-[color:var(--color-muted)]">—</td>
                  <td className="px-5 py-3 text-[color:var(--color-muted)]">—</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </PageContainer>
  )
}

function FlowsRoute() {
  return (
    <ErrorBoundary>
      <FlowsTable />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/flows/')({ component: FlowsRoute })
