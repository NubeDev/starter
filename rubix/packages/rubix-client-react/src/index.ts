// # @nube/rubix-client-react
//
// React bindings for `@nube/rubix-client-ts`. This package will grow
// typed hooks per rubix-agent endpoint family (auth, users,
// extensions, …) in later stages; for now it ships only the
// provider scaffold so downstream packages and the rubix frontend
// can wire context at the app root.
//
// `RubixClientProvider` mounts a sibling `StarterClientProvider`
// under the hood with the wrapped client's `.starter` instance, so
// hooks from both `@nube/starter-client-react` and
// `@nube/rubix-client-react` resolve against the same long-lived
// transport.

export {
  RubixClientProvider,
  useRubixClient,
} from "./provider/rubix-client-provider.js";
export type { RubixClientProviderProps } from "./provider/rubix-client-provider.js";

export * from "./hooks/system.js";
export * from "./hooks/users.js";
export * from "./hooks/mcp.js";
export * from "./hooks/extensions.js";
export * from "./hooks/use-extension-events.js";
export * from "./hooks/teams.js";
export * from "./hooks/tenants.js";
export * from "./hooks/clickhouse.js";
export * from "./hooks/flow-ops.js";
export * from "./hooks/flow-events.js";
export * from "./hooks/dashboard.js";
export * from "./hooks/use-dashboard-sidebar.js";
export * from "./hooks/use-page-liveness.js";
export * from "./hooks/undo.js";
export * from "./hooks/audit.js";
export * from "./hooks/insights.js";
