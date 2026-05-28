import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/extensions/$extId/$')({
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/extensions/$extId/$"!</div>
}
