import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/admin/access')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/admin/access"!</div>
}
