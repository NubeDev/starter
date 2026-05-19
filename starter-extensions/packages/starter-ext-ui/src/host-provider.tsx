// `ExtensionHostProvider` — the one provider a host shell mounts
// around its UI tree to enable extensions.
//
// Wires two contexts in one place:
//
// 1. `ExtensionHostContext` (from this package) — the manager
//    `<ExtensionSlot/>` and `useExtensionHost()` consume.
// 2. `ExtensionHostClientContext` (from `@nube/starter-ext-sdk-ts`)
//    — the typed `StarterClient` extensions read via
//    `useHostClient()`. SCOPE R11: UI extensions never raw fetch.
//
// Keeping both providers behind one component means the host shell
// has exactly one wiring point and cannot accidentally enable one
// without the other.

import * as React from "react";

import { ExtensionHostClientProvider } from "@nube/starter-ext-sdk-ts";

import { ExtensionHostContext } from "./host-context.js";
import type { ExtensionHostManager } from "./host-manager.js";

export interface ExtensionHostProviderProps {
  host: ExtensionHostManager;
  children: React.ReactNode;
}

export function ExtensionHostProvider(
  props: ExtensionHostProviderProps,
): React.ReactElement {
  return (
    <ExtensionHostContext.Provider value={props.host}>
      <ExtensionHostClientProvider client={props.host.client}>
        {props.children}
      </ExtensionHostClientProvider>
    </ExtensionHostContext.Provider>
  );
}
