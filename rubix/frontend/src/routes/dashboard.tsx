// `/dashboard` legacy route — Phase D.2 moved authoring to
// `/dashboards/$pageId`. This file preserves the old URL by
// redirecting to the bundled `disk-overview` worked example.

import { createFileRoute, redirect } from '@tanstack/react-router'

export const Route = createFileRoute('/dashboard')({
  beforeLoad: () => {
    throw redirect({ to: '/dashboards/$pageId', params: { pageId: 'disk-overview' } })
  },
})
