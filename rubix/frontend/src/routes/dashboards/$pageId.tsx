// `/dashboards/$pageId` — mounts the SDUI renderer for one page.
//
// `<SduiPage pageRef=...>` reads `<SduiProvider>` from `main.tsx`
// and runs the resolve / subscribe / action loop end-to-end. The
// route file itself owns no rendering logic — every widget comes
// out of `@nube/starter-ui-sdui-react`'s per-variant renderers.

import { createFileRoute, Link } from '@tanstack/react-router'
import { SduiPage } from '@nube/starter-ui-sdui-react'
import { ErrorBoundary } from '@/components/error-boundary'

function DashboardPageRoute() {
  const { pageId } = Route.useParams()
  // Bundled pages are stored as `dashboard.<slug>`; the URL carries
  // only the slug for cleaner deep links. Hand-authored ids that
  // already include a dot are passed through verbatim.
  const pageRef = pageId.includes('.') ? pageId : `dashboard.${pageId}`
  return (
    <ErrorBoundary>
      <section className="relative mx-auto max-w-7xl px-4 pb-24 pt-6 sm:px-6 lg:px-8">
        <div className="mb-3 flex justify-end">
          <Link
            to="/dashboards/$pageId/edit"
            params={{ pageId }}
            className="rounded-md border border-slate-300 px-3 py-1 text-sm hover:bg-slate-50"
          >
            Edit
          </Link>
        </div>
        <SduiPage pageRef={pageRef} />
      </section>
    </ErrorBoundary>
  )
}

export const Route = createFileRoute('/dashboards/$pageId')({ component: DashboardPageRoute })
