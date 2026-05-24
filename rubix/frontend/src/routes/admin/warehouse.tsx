// `/admin/warehouse` — stub for the Phase C ClickHouse + insights
// admin surface. The real panel will mount here once the warehouse
// management API + insights mart configuration land. The stub
// exists today so the left-nav Admin section in
// `src/lib/nav.ts` resolves to a real route and so operators can
// discover where the surface will live.

import { createFileRoute } from '@tanstack/react-router'
import { useIntl } from 'react-intl'
import { Database } from 'lucide-react'
import { ErrorBoundary } from '@/components/error-boundary'

function WarehousePanel() {
  const intl = useIntl()
  const tr = (id: string, def: string) =>
    intl.formatMessage({ id, defaultMessage: def })

  return (
    <section className="relative mx-auto max-w-5xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
      <header className="mb-8">
        <div className="flex items-center gap-3">
          <span className="h-px w-8 bg-[color:var(--color-leaf)]" />
          <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-[color:var(--color-leaf)]">
            {tr('warehouse.eyebrow', 'Admin')}
          </span>
        </div>
        <h1 className="mt-3 text-4xl font-medium tracking-[-0.03em]">
          {tr('warehouse.title', 'Warehouse')}
        </h1>
      </header>

      <div className="glass flex flex-col items-start gap-3 rounded-3xl p-8">
        <Database className="h-6 w-6 text-[color:var(--color-leaf)]" />
        <p className="text-sm text-[color:var(--color-muted)]">
          {tr(
            'warehouse.stub.body',
            'ClickHouse + insights admin lands here in Phase C. This page exists so the Admin nav section resolves to a real route today.',
          )}
        </p>
      </div>
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
