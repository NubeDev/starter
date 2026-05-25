// `/dashboards` — list of authored dashboards.
//
// Reads `useDashboardList()` from `@nube/rubix-client-react`. Each
// row is a link to `/dashboards/$pageId`. Loading and empty states
// reuse the same primitives as `/flows`.

import { createFileRoute, Link } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import { LayoutDashboard } from 'lucide-react'
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
  Skeleton,
} from '@nube/starter-ui-kit'
import { useDashboardList } from '@nube/rubix-client-react'
import { ErrorBoundary } from '@/components/error-boundary'

function DashboardsTable() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })
  const list = useDashboardList()
  const rows = list.data?.pages ?? []

  return (
    <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-8">
        <div className="flex items-center gap-3">
          <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
            {tr('dashboards.eyebrow', 'Dashboards')}
          </span>
        </div>
        <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
          {tr('dashboards.title', 'Authored dashboards')}
        </h1>
        <p className="mt-2 max-w-2xl text-sm text-[color:var(--color-muted)]">
          {tr(
            'dashboards.subtitle',
            'Every SDUI page with a live revision. Click a row to open it; the dashboard-assistant flow can edit any of them.',
          )}
        </p>
      </header>

      {list.isLoading ? (
        <div className="space-y-3">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : rows.length === 0 ? (
        <Empty>
          <EmptyHeader>
            <EmptyMedia>
              <LayoutDashboard className="h-8 w-8" />
            </EmptyMedia>
            <EmptyTitle>{tr('dashboards.empty.title', 'No dashboards yet')}</EmptyTitle>
            <EmptyDescription>
              {tr(
                'dashboards.empty.description',
                'Ask the dashboard-assistant to create one, or seed the bundled disk-overview page.',
              )}
            </EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <table className="w-full text-sm">
          <thead className="text-[11px] uppercase tracking-[0.18em] text-[color:var(--color-subtle)]">
            <tr>
              <th className="py-2 text-left">{tr('dashboards.col.pageId', 'Page id')}</th>
              <th className="py-2 text-left">{tr('dashboards.col.title', 'Title')}</th>
              <th className="py-2 text-left">{tr('dashboards.col.owner', 'Owner')}</th>
              <th className="py-2 text-left">{tr('dashboards.col.revision', 'Revision')}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.page_id} className="border-t border-[color:var(--color-border)]">
                <td className="py-2">
                  <Link
                    to="/dashboards/$pageId"
                    params={{ pageId: row.page_id.replace(/^dashboard\./, '') }}
                    className="text-[color:var(--color-leaf)] hover:underline"
                  >
                    {row.page_id}
                  </Link>
                </td>
                <td className="py-2">{row.title}</td>
                <td className="py-2">{row.owner_principal}</td>
                <td className="py-2 font-mono text-xs">{row.revision_id.slice(0, 8)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  )
}

function DashboardsRoute() {
  return (
    <ErrorBoundary>
      <DashboardsTable />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/dashboards/')({ component: DashboardsRoute })
