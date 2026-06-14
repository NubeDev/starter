import { QueryClient } from "@tanstack/react-query";
import { StarterError } from "@nube/starter-client-ts";

// One ambient QueryClient for the whole app — also the federation
// singleton the host shares with extensions (one cache across host ↔
// remotes). Defaults mirror `@nube/starter-client-react`'s QueryProvider
// (30s stale, 5min gc) but auth failures never retry: a 401/403 is a
// session state, not a transient error.
export function createNexusQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        gcTime: 5 * 60_000,
        retry: (failureCount, error) => {
          if (
            error instanceof StarterError &&
            (error.status === 401 || error.status === 403)
          ) {
            return false;
          }
          return failureCount < 2;
        },
      },
    },
  });
}
