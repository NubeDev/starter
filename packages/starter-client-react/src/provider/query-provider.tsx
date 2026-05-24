// `QueryProvider` — thin opinionated wrapper around TanStack
// Query's `QueryClientProvider`.
//
// Defaults chosen to match the starter contract: dashboards refresh
// often enough that 30s staleTime feels live, 5min gcTime keeps
// recently-viewed pages instant on back-nav, and the retry rule
// short-circuits on 401/403 — those are auth states, not transient
// failures, so retrying just delays the user reaching /login.
//
// Apps that need different behaviour can pass their own pre-built
// `QueryClient` via the `client` prop; the defaults only apply when
// we construct the client ourselves.

import { useState, type ReactNode } from "react";
import {
  QueryClient,
  QueryClientProvider,
  type QueryClientConfig,
} from "@tanstack/react-query";
import { StarterError } from "@nube/starter-client-ts";

export interface QueryProviderProps {
  /** Optional pre-built `QueryClient`. Bring your own to override defaults. */
  client?: QueryClient;
  children: ReactNode;
}

const defaultConfig: QueryClientConfig = {
  defaultOptions: {
    queries: {
      staleTime: 30 * 1000,
      gcTime: 5 * 60 * 1000,
      retry: (failureCount, error) => {
        if (error instanceof StarterError) {
          if (error.status === 401 || error.status === 403) return false;
        }
        return failureCount < 3;
      },
    },
  },
};

export function QueryProvider(props: QueryProviderProps) {
  // `useState` so the client survives re-renders without being
  // re-instantiated each pass.
  const [client] = useState(() => props.client ?? new QueryClient(defaultConfig));
  return <QueryClientProvider client={client}>{props.children}</QueryClientProvider>;
}
