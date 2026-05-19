// `useHostClient` — typed wrapper over `@nube/starter-client-ts`.
//
// SCOPE R11 forbids UI extensions from issuing raw `fetch` calls.
// Every host call goes through the `StarterClient` instance the
// host injects via `ExtensionHostClientProvider`. Auth, tracing,
// and retry policy live in the client; extensions consume them by
// composition, never by re-implementing them.
//
// The provider is host-side (mounted by `@nube/starter-ext-ui`'s
// `ExtensionHostProvider`); the hook is what the extension calls.
// Splitting provider + hook into this package means an extension
// author can mock the host client in a unit test without pulling
// the federation runtime.

import * as React from "react";
import type { StarterClient } from "@nube/starter-client-ts";

/**
 * The handle a UI extension receives from `useHostClient()`.
 *
 * In v0.1 this is exactly `StarterClient` from
 * `@nube/starter-client-ts` — the typed wrapper exists so that future
 * additions (per-extension scoping, telemetry tags, request-id
 * propagation) can be layered without changing the call sites in
 * extension code.
 */
export type ExtensionHostClient = StarterClient;

const HostClientContext = React.createContext<ExtensionHostClient | null>(null);

export interface ExtensionHostClientProviderProps {
  /** The host's `StarterClient` instance. */
  client: ExtensionHostClient;
  children: React.ReactNode;
}

/**
 * Host-side provider. Mounted once by the host shell; every
 * `<ExtensionSlot/>` descends from it so panels can call
 * `useHostClient()` without per-extension wiring.
 */
export function ExtensionHostClientProvider(
  props: ExtensionHostClientProviderProps,
): React.ReactElement {
  return (
    <HostClientContext.Provider value={props.client}>
      {props.children}
    </HostClientContext.Provider>
  );
}

/**
 * Read the host's RPC client. Throws if called outside an
 * `ExtensionHostClientProvider` — that's a wiring bug in the host
 * shell, not something the extension can recover from.
 */
export function useHostClient(): ExtensionHostClient {
  const client = React.useContext(HostClientContext);
  if (!client) {
    throw new Error(
      "useHostClient() called outside <ExtensionHostClientProvider>. " +
        "The host shell must wrap extension slots in ExtensionHostProvider.",
    );
  }
  return client;
}
