// Auto-load every enabled, UI-contributing extension once the user
// is authenticated.
//
// Without this, an operator has to visit `/extensions` and press
// "Load UI" per row before any `<ExtensionSlot/>` mount would render
// — including the AppSidebar's `sidebar-nav` and `sidebar` slots,
// which are visible on every authed route. Wiring `bootstrapExtensions`
// at app boot makes UI contributions show up as soon as the user lands
// on any page.
//
// Why gated on `isAuthenticated`: `GET /api/v1/extensions` is an
// Admin-only route — calling it before login 401s and bootstraps
// nothing. We wait until AuthProvider reports a session, then run
// once. Subsequent re-renders short-circuit on the `done` ref.

import * as React from 'react'

import { useAuth } from '@nube/starter-client-react'
import { bootstrapExtensions } from '@nube/starter-ext-ui'

import { getExtensionHost } from './extension-host'

export function ExtensionAutoLoader(): null {
  const { isAuthenticated } = useAuth()
  const done = React.useRef(false)

  React.useEffect(() => {
    if (!isAuthenticated || done.current) return
    done.current = true
    void bootstrapExtensions(getExtensionHost(), {
      basePath: '/api/v1/extensions',
      onRegistered: (id) => {
        // eslint-disable-next-line no-console
        console.info(`[rubix.extensions] auto-loaded UI for ${id}`)
      },
    }).catch((err: unknown) => {
      // eslint-disable-next-line no-console
      console.warn('[rubix.extensions] bootstrap failed:', err)
      done.current = false
    })
  }, [isAuthenticated])

  return null
}
