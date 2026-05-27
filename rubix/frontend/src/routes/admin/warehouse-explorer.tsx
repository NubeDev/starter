// `/admin/warehouse-explorer` — visual rebuild of the warehouse
// explorer, forked from sql-studio. Lives at a sibling URL to
// `/admin/warehouse` (not nested as a tab) and takes the full page
// width. The library at `@nube/starter-ui-warehouse-explorer` carries
// its own header / nav bar; no outer page chrome here.
//
// Providers: relies on the rubix app-root `QueryClientProvider` and
// `StarterClientProvider` from `main.tsx` — the explorer hooks read
// the ambient `StarterClient` via `useStarterClient()`.

import { createFileRoute } from '@tanstack/react-router'
import {
  Explorer,
  SqlProvider,
} from '@nube/starter-ui-warehouse-explorer'
import '@nube/starter-ui-warehouse-explorer/theme.css'
import { ErrorBoundary } from '@/components/error-boundary'

function WarehouseExplorerRoute() {
  return (
    <ErrorBoundary>
      <SqlProvider>
        <Explorer />
      </SqlProvider>
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/warehouse-explorer')({
  component: WarehouseExplorerRoute,
})
