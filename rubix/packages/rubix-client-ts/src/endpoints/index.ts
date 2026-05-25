// Barrel of endpoint modules. Each module augments `RubixClient`
// with the methods for one verb family — mirrors the Rust
// `rubix-client` crate's per-area files (system, alert, ...).
//
// Audit is intentionally absent from this barrel (and from this
// package): the `/v1/audit` read route lives on starter-server (see
// `crates/starter-audit/src/routes.rs`), so audit reads belong on
// `@nube/starter-client-ts`, not here. Tracked as SCOPE OQ-3.

export * from "./system.js";
export * from "./alert.js";
export * from "./user.js";
export * from "./team.js";
export * from "./tenant.js";
export * from "./clickhouse.js";
export * from "./flow_ops.js";
export * from "./dashboard.js";
export * from "./undo.js";
export * from "./mcp.js";
