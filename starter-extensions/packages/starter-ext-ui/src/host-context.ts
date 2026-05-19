// React context plumbing for `ExtensionHostManager`. Split out into
// its own file so the provider component and the hooks can both
// import it without circular references.

import * as React from "react";

import type { ExtensionHostManager } from "./host-manager.js";

export const ExtensionHostContext =
  React.createContext<ExtensionHostManager | null>(null);

/**
 * Read the host manager. Throws when called outside a provider —
 * that's a host-shell wiring bug, surfaced through the nearest
 * error boundary.
 */
export function useExtensionHostManager(): ExtensionHostManager {
  const mgr = React.useContext(ExtensionHostContext);
  if (!mgr) {
    throw new Error(
      "useExtensionHostManager() called outside <ExtensionHostProvider>.",
    );
  }
  return mgr;
}
