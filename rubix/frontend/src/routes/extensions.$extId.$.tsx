// `/extensions/$extId/$rest` — per-extension routed view.
//
// Mounts the `main` slot for one extension (filtered by extensionId)
// and pipes the catch-all `$rest` path tail into `SlotContext.route`,
// so the extension can render different views per sub-path without
// the host owning a route table for each extension.
//
// URL examples:
//   /extensions/com.rubix.example/                         → route = ""
//   /extensions/com.rubix.example/customers/by-country     → route = "customers/by-country"
//   /extensions/com.rubix.example/products/low-stock       → route = "products/low-stock"
//
// The extension reads the tail with `useExtensionRoute()` from
// `@nube/starter-ext-sdk-ts`. The admin index page (`/extensions`)
// remains the fleet-wide list; this route is the per-extension view
// nav-tree items deep-link into.

import { createFileRoute } from '@tanstack/react-router'

import { ExtensionSlot } from '@nube/starter-ext-ui'

import { ErrorBoundary } from '@/components/error-boundary'

function ExtensionRoute() {
  const { extId, _splat } = Route.useParams()
  const route = _splat ?? ''
  return (
    <ErrorBoundary>
      <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
        <ExtensionSlot id="main" extensionId={extId} route={route} />
      </section>
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/extensions/$extId/$')({
  component: ExtensionRoute,
})
