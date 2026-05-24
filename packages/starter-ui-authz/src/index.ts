// @nube/starter-ui-authz — admin UI for tenants, teams, members,
// authz rules, assignments, and the decisions audit feed.
//
// Mount `<AuthzAdmin>` inside `<StarterClientProvider>` +
// `<QueryProvider>` (both from `@nube/starter-client-react`).
// Individual panels can also be imported from `./panels` for
// hosts wanting a different layout.

export * from "./panels/index.js";
export * from "./hooks/index.js";
export * from "./i18n/index.js";
