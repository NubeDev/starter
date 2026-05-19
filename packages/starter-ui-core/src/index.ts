// # @nube/starter-ui-core
//
// React glue for starter. Three thin pieces:
//
// - `auth/` — `<AuthProvider>` + `useAuth()` with pluggable strategies
//   (session cookie, bearer token, external IdP). Identical hook
//   surface across modes so app code doesn't branch.
// - `query/` — `starterQueryKey(...)` helper so every starter-owned
//   react-query key is namespaced under `['starter', ...]`.
// - `testing/` — `MockServer` + `renderWithProviders` for consumer tests.

export * from "./auth/index.js";
export * from "./query/index.js";
