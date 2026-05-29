// Per-extension page route.
//
// Matches `/extensions/:extId/*rest`. The host doesn't know what
// the extension wants to render — it just mounts the `main` slot
// for that one extension via `<ExtensionSlot/>` and forwards the
// splat as `route`. The extension's own `Main` component then
// dispatches on `useExtensionRoute()` to pick a sub-page.
//
// Extensions are auto-loaded at boot by `ExtensionAutoLoader`
// (see `src/lib/extension-autoloader.tsx`), so by the time the
// user clicks an extension nav link the slot resolver already has
// a `Main` component ready to mount.

import { createFileRoute } from '@tanstack/react-router'
import { ExtensionSlot } from '@nube/starter-ext-ui'

export const Route = createFileRoute('/extensions/$extId/$')({
  component: RouteComponent,
})

function RouteComponent() {
  const { extId, _splat } = Route.useParams()
  return (
    <ExtensionSlot
      id="main"
      extensionId={extId}
      route={_splat ?? ''}
    />
  )
}
