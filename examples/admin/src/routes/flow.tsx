import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/flow')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/flow"!</div>
}
