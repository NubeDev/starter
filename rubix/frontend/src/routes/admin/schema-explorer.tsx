// `/admin/schema-explorer` — the re-skinned schema view (polished
// schema-viewer look: breadcrumb header, left schema tree, ERD canvas
// with a floating toolbar). Sibling to `/admin/warehouse-explorer`,
// which keeps the original sql-studio explorer untouched.
//
// Providers: relies on the rubix app-root `QueryClientProvider` and
// `StarterClientProvider` from `main.tsx` — the explorer hooks read the
// ambient `StarterClient` via `useStarterClient()`.

import { createFileRoute } from '@tanstack/react-router'
import { SchemaExplorer } from '@nube/starter-ui-warehouse-explorer'
import '@nube/starter-ui-warehouse-explorer/theme.css'
import { ErrorBoundary } from '@/components/error-boundary'

function SchemaExplorerRoute() {
  return (
    <ErrorBoundary>
      <SchemaExplorer />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/schema-explorer')({
  component: SchemaExplorerRoute,
})
