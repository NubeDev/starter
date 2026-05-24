// `RubixClientProvider` — shares one long-lived `RubixClient` with
// descendants via React context, and mounts a sibling
// `StarterClientProvider` for the wrapped `.starter` transport so
// hooks from `@nube/starter-client-react` (auth, useEventStream)
// resolve against the same client without the app having to wire
// both providers by hand.
//
// Construct one `RubixClient` per app at the top level (typically
// in `main.tsx`) and pass it to this provider. Typed hooks added in
// later stages (users, extensions, …) reach for the same client
// through `useRubixClient`, so a single instance backs every hook
// in the tree.

import { createContext, useContext, type ReactNode } from "react";
import { RubixClient } from "@nube/rubix-client-ts";
import { StarterClientProvider } from "@nube/starter-client-react";

const RubixClientContext = createContext<RubixClient | null>(null);

export interface RubixClientProviderProps {
  /** The single `RubixClient` instance to share. */
  client: RubixClient;
  children: ReactNode;
}

/**
 * Provide a `RubixClient` instance to descendants, and the wrapped
 * `StarterClient` to any `@nube/starter-client-react` hooks via a
 * nested `StarterClientProvider`.
 *
 * Place near the root of the app — above `QueryProvider`,
 * `AuthProvider`, and any routes that call typed hooks.
 */
export function RubixClientProvider(props: RubixClientProviderProps) {
  return (
    <RubixClientContext.Provider value={props.client}>
      <StarterClientProvider client={props.client.starter}>
        {props.children}
      </StarterClientProvider>
    </RubixClientContext.Provider>
  );
}

/**
 * Read the ambient `RubixClient`. Throws if no provider is
 * mounted — that's a programming error, not a runtime condition, so
 * we fail loudly rather than returning `null` and letting callers
 * unwrap silently.
 */
export function useRubixClient(): RubixClient {
  const client = useContext(RubixClientContext);
  if (!client) {
    throw new Error(
      "useRubixClient() called outside <RubixClientProvider>. " +
        "Mount the provider near your app root with a constructed RubixClient.",
    );
  }
  return client;
}
