// `StarterClientProvider` — shares one long-lived `StarterClient`
// with descendant hooks via React context.
//
// The client itself is transport-only (see `@nube/starter-client-ts`):
// it holds a base URL, a `fetch` reference, and default headers. We
// keep React out of that package on purpose so non-React consumers
// (CLI scripts, server-side codegen) can use it too. This wrapper is
// where React enters the picture.
//
// Construct one `StarterClient` per app at the top level (typically
// in `main.tsx`) and pass it to this provider. Sibling React
// packages (`@nube/rubix-client-react`, etc.) all reach for the same
// client through `useStarterClient`, so a single instance backs every
// hook in the tree.

import { createContext, useContext, type ReactNode } from "react";
import { StarterClient } from "@nube/starter-client-ts";

const StarterClientContext = createContext<StarterClient | null>(null);

export interface StarterClientProviderProps {
  /** The single `StarterClient` instance to share. */
  client: StarterClient;
  children: ReactNode;
}

/**
 * Provide a `StarterClient` instance to descendants.
 *
 * Place near the root of the app — above `QueryProvider` and any
 * routes that call typed hooks.
 */
export function StarterClientProvider(props: StarterClientProviderProps) {
  return (
    <StarterClientContext.Provider value={props.client}>
      {props.children}
    </StarterClientContext.Provider>
  );
}

/**
 * Read the ambient `StarterClient`. Throws if no provider is
 * mounted — that's a programming error, not a runtime condition, so
 * we fail loudly rather than returning `null` and letting callers
 * unwrap silently.
 */
export function useStarterClient(): StarterClient {
  const client = useContext(StarterClientContext);
  if (!client) {
    throw new Error(
      "useStarterClient() called outside <StarterClientProvider>. " +
        "Mount the provider near your app root with a constructed StarterClient.",
    );
  }
  return client;
}
