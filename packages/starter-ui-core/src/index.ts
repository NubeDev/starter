// # @nube/starter-ui-core
//
// React glue for starter. Three thin pieces:
//
// - `auth/` — `<AuthProvider>` + `useAuth()` with pluggable strategies
//   (session cookie, bearer token, external IdP). Identical hook
//   surface across modes so app code doesn't branch.
// - `query/` — `starterQueryKey(...)` helper so every starter-owned
//   react-query key is namespaced under `['starter', ...]`.
// - `testing/` — `createMockServer` (fetch shim) and `createAuthWrapper`
//   (component wrapper for RTL-style render). Imported via the
//   `@nube/starter-ui-core/testing` subpath to keep test-only code out
//   of production bundles.

export * from "./auth/index.js";
export * from "./query/index.js";
export * from "./theme-editor/index.js";
export * from "./preferences/index.js";
