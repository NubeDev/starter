// `/admin/users` legacy route — the user-management surface moved
// into `/admin/access` (Users rail node + Profile tab on user
// detail). This file preserves the old URL by redirecting.

import { createFileRoute, redirect } from '@tanstack/react-router'

export const Route = createFileRoute('/admin/users')({
  beforeLoad: () => {
    throw redirect({ to: '/admin/access' })
  },
})
