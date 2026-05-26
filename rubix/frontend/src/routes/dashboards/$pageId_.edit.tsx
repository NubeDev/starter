import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/dashboards/$pageId/edit')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/dashboards/$pageId/edit"!</div>
}
