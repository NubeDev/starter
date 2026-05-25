// Extension Module-Federation host wiring.
//
// Constructs a single process-wide `ExtensionHostManager` bound to
// the shared `StarterClient`, with the four well-known singletons
// (`react`, `react-dom`, `@tanstack/react-query`, `zustand`) bound
// to the rubix-frontend's own live instances. Extensions that
// declare matching majors receive these references at `init`-time
// and the host enforces a hard refusal on mismatch — per
// `starter-extensions/DOCS/extensions/scope/SCOPE.md` §"Decisions
// made / singleton-mismatch".
//
// The manager itself is owned upstream
// (`@nube/starter-ext-ui::ExtensionHostManager`); rubix-frontend
// just composes it with our concrete deps, which is the
// consumer's job per rubix SCOPE R2.

import * as React from 'react'
import * as ReactDOM from 'react-dom'
import * as ReactQuery from '@tanstack/react-query'
import * as Zustand from 'zustand'

import {
  ExtensionHostManager,
  type ExtensionHostManagerOptions,
} from '@nube/starter-ext-ui'

import { getStarterClient } from './client'

let cached: ExtensionHostManager | null = null

/** Process-wide `ExtensionHostManager`. Lazy so test paths that
 * never mount the provider don't construct it. */
export function getExtensionHost(): ExtensionHostManager {
  if (cached) return cached
  const opts: ExtensionHostManagerOptions = {
    client: getStarterClient(),
    singletons: {
      react: { version: React.version, instance: React },
      'react-dom': { version: ReactDOM.version, instance: ReactDOM },
      '@tanstack/react-query': {
        // `@tanstack/react-query` ships its version on the package
        // root; in tree-shaken builds it falls back to the major.
        version: (ReactQuery as unknown as { version?: string }).version ?? '5',
        instance: ReactQuery,
      },
      zustand: {
        version:
          (Zustand as unknown as { version?: string }).version ?? '5',
        instance: Zustand,
      },
    },
    telemetry: (ev) => {
      // Surface mismatches in dev so an author notices immediately.
      // Production telemetry can replace this when an event bus
      // lands.
      // eslint-disable-next-line no-console
      console[ev.severity === 'error' ? 'error' : 'warn'](
        '[rubix.extensions]',
        ev,
      )
    },
  }
  cached = new ExtensionHostManager(opts)
  return cached
}
