// `/admin/access` — master-detail Access Control surface. The
// heavy lifting (URL <-> selection mapping, i18n adapter, host
// `userDirectory`/`userOps` adapters) lives in
// `@/lib/access-control` so the catch-all sibling at
// `access.$.tsx` can mount the same component for deep links such
// as `/admin/access/t/<slug>/team/<slug>`.

import { createFileRoute } from '@tanstack/react-router'
import { AccessControl } from '@/lib/access-control'
import { ErrorBoundary } from '@/components/error-boundary'

function AccessRoute() {
  return (
    <ErrorBoundary>
      <AccessControl splat="" />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/access')({ component: AccessRoute })
