// `/admin/access/*` — catch-all that powers deep links into the
// master-detail Access Control surface. Examples:
//   /admin/access/t/acme
//   /admin/access/t/acme/team/platform
//   /admin/access/t/acme/u/<userId>
//   /admin/access/u/<userId>
//
// Slug-to-id resolution and the reverse `SelectedNode -> path`
// mapping both live in `@/lib/access-control`.

import { createFileRoute } from '@tanstack/react-router'
import { AccessControl } from '@/lib/access-control'
import { ErrorBoundary } from '@/components/error-boundary'

function AccessSplatRoute() {
  const { _splat } = Route.useParams() as { _splat?: string }
  return (
    <ErrorBoundary>
      <AccessControl splat={_splat ?? ''} />
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/admin/access/$')({
  component: AccessSplatRoute,
})
