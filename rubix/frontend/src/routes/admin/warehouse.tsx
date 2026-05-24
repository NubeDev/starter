// `/admin/warehouse` — ClickHouse + insights admin. Mounts the
// rubix-side `<WarehouseAdmin>` tabbed shell from
// `@/components/admin/warehouse`. The shell composes rules, marts,
// retention, and insights panels — each reads/writes via
// `@nube/rubix-client-react` hooks against `rubix.clickhouse.*` and
// `rubix.insights.*` tools.

import { createFileRoute } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import { WarehouseAdmin } from '@/components/admin/warehouse'
import { ErrorBoundary } from '@/components/error-boundary'

function WarehousePanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  return (
    <section className="relative mx-auto max-w-6xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-8">
        <div className="flex items-center gap-3">
          <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
            {tr('admin.warehouse.eyebrow', 'Admin')}
          </span>
        </div>
        <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
          {tr('admin.warehouse.title', 'Warehouse')}
        </h1>
        <p className="mt-2 max-w-2xl text-sm text-[color:var(--color-muted)]">
          {tr(
            'admin.warehouse.subtitle',
            'ClickHouse projection rules, materialised marts, retention policy, and insights rules.',
          )}
        </p>
      </header>
      <WarehouseAdmin />
    </section>
  )
}

function WarehouseRoute() {
  return (
    <ErrorBoundary>
      <WarehousePanel />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/warehouse')({
  component: WarehouseRoute,
})
