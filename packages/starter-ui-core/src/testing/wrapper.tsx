// Test wrapper factory. Returns a `({ children }) => ReactNode`
// suitable for `render(ui, { wrapper })` in @testing-library/react
// (or any equivalent). We do NOT import RTL here — that's a consumer
// choice, and a peer is the wrong shape (RTL is a devDep, not a peer).
//
// Wraps the children in:
//   <QueryClientProvider> → react-query for any starter hooks
//     <AuthProvider>      → useAuth() works
//       {children}
//
// Each call creates a fresh `QueryClient` (cache isolation between
// tests) unless the caller passes their own.

import type { ComponentType, ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { StarterClient } from "@nube/starter-client-ts";

import { AuthProvider } from "../auth/provider.js";
import type { AuthStrategy } from "../auth/strategy.js";

export interface AuthWrapperOptions {
  client: StarterClient;
  strategy: AuthStrategy;
  /** Override the auto-created QueryClient. Useful when a test wants
   *  to inspect cache state. */
  queryClient?: QueryClient;
}

/** Build the wrapper component. */
export function createAuthWrapper(opts: AuthWrapperOptions): ComponentType<{ children: ReactNode }> {
  const qc =
    opts.queryClient ??
    new QueryClient({
      // Tests should see retries fail fast and no background refetches.
      defaultOptions: {
        queries: { retry: false, refetchOnWindowFocus: false, staleTime: 0 },
        mutations: { retry: false },
      },
    });

  return function StarterTestWrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={qc}>
        <AuthProvider client={opts.client} strategy={opts.strategy}>
          {children}
        </AuthProvider>
      </QueryClientProvider>
    );
  };
}
