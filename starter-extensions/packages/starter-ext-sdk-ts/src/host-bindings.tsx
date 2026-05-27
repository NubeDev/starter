// `HostBindingsContext` — the internal React context that carries
// the host's resolved singletons (and the calling extension's id)
// down to the prefs/i18n/formatters hooks.
//
// Why a React context instead of a module-level stash: tests need to
// stand up multiple panels under different mock hosts in the same
// process (`MockHostProvider`), and the SDK is used by multiple
// extensions in the same page in production. Module-level state
// would race; a React context is per-subtree and matches React's
// own provider semantics for the rest of the SDK.
//
// In production, `registerExtensionContributions` wraps every
// contributed component in a `<HostBindingsProvider>` seeded from
// `handle.id` + `handle.singletons` — extension authors never see
// this provider, but they get hooks that "just work".
//
// In tests, `<MockHostProvider>` mounts the same provider directly,
// using whichever Context objects (real or fake) the test passes in
// — see `./testing/mock-host-provider.tsx`.

import * as React from "react";

import type { ResolvedSingletons } from "./register.js";

export interface HostBindings {
  /** Extension id (reverse-DNS, matches `block.yaml`). Used by
   *  `useHostTranslate` to auto-prefix keys lacking a dot. */
  extensionId: string;
  /** Resolved singletons the host negotiated. Keyed by the package
   *  name the extension declared in its `RemoteFactory.singletons`. */
  singletons: ResolvedSingletons;
}

// See `slot-context.tsx` for the rationale of the `globalThis` stash:
// multiple bundled copies of this module must share one context
// instance, otherwise `useContext` returns `null` for extensions
// whose remote bundle is dynamically imported into the host page.
const HOST_BINDINGS_CTX_KEY = "__starterExtSdkHostBindingsContextV1";
const HostBindingsContext: React.Context<HostBindings | null> =
  ((globalThis as unknown as Record<string, unknown>)[
    HOST_BINDINGS_CTX_KEY
  ] as React.Context<HostBindings | null> | undefined) ??
  (((globalThis as unknown as Record<string, unknown>)[
    HOST_BINDINGS_CTX_KEY
  ] = React.createContext<HostBindings | null>(null)) as React.Context<
    HostBindings | null
  >);

export interface HostBindingsProviderProps {
  bindings: HostBindings;
  children: React.ReactNode;
}

/**
 * Internal SDK provider. Production code reaches it via
 * `registerExtensionContributions`; tests reach it via
 * `<MockHostProvider>`. Application code should not mount this
 * directly — the bindings carry the host's React Context objects
 * (singletons), which only the federation runtime can produce.
 */
export function HostBindingsProvider(
  props: HostBindingsProviderProps,
): React.ReactElement {
  return (
    <HostBindingsContext.Provider value={props.bindings}>
      {props.children}
    </HostBindingsContext.Provider>
  );
}

/**
 * Read the bindings. Throws when called outside a
 * `<HostBindingsProvider>` — that means the panel was rendered
 * without the federation wrapper (production bug) or without
 * `<MockHostProvider>` (test wiring bug). The host bug class is
 * loud, not silent.
 */
export function useHostBindings(): HostBindings {
  const ctx = React.useContext(HostBindingsContext);
  if (!ctx) {
    throw new Error(
      "useHostBindings(): no <HostBindingsProvider> in the tree. " +
        "This panel must be rendered by the host's federation runtime " +
        "(production) or under <MockHostProvider> (tests).",
    );
  }
  return ctx;
}
