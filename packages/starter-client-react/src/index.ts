// # @nube/starter-client-react
//
// React bindings for `@nube/starter-client-ts`. Provides:
//
// - `StarterClientProvider` + `useStarterClient` — make a long-lived
//   `StarterClient` available through context to descendant hooks.
// - `QueryProvider` — a thin opinionated wrapper around TanStack
//   Query's `QueryClientProvider` with starter-defaults (30s
//   staleTime, 5min gcTime, retry that skips on 401/403).
//
// Endpoint-shaped hooks (users, auth, etc.) live in sibling typed
// packages (e.g. `@nube/rubix-client-react`) — this package stays
// transport-only.

export { StarterClientProvider, useStarterClient } from "./provider/starter-client-provider.js";
export { QueryProvider } from "./provider/query-provider.js";
export type { QueryProviderProps } from "./provider/query-provider.js";
export { AuthProvider, useAuth, ME_QUERY_KEY } from "./provider/auth-provider.js";
export type { AuthContextValue, AuthProviderProps } from "./provider/auth-provider.js";
export { useEventStream } from "./hooks/use-event-stream.js";
export type {
  EventStreamStatus,
  UseEventStreamOptions,
  UseEventStreamResult,
} from "./hooks/use-event-stream.js";
