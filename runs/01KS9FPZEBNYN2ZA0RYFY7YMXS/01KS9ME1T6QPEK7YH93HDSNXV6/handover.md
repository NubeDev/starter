## Done

- DecisionEntry gained `surface: Option<String>` + sqlite/postgres migration 0005 adding nullable `surface TEXT` column. Engine reads value from a new `starter_authz::with_surface` tokio task-local.
- REST middleware (`with_permission` / `with_permission_owned`) wraps its engine.check in `with_surface("rest", …)`.
- MCP parity: new `AuthzedToolBinding` in starter-ext-mcp; `register_tools_with_engine` / `register_process_tools_with_engine` wrap tools whose manifest declared `auth.permission`. Principal flows via new `starter_mcp::principal_local` task-local bound by the MCP HTTP `auth_layer`.
- gRPC parity: `ExtensionGrpcService::with_authz(engine, authenticator)` adds the first authz layer on the gRPC backplane; reads bearer from `authorization` metadata, runs check in `with_surface("grpc", …)`. `GrpcMethod` carries `permission: Option<PermissionGate>` from the manifest.
- All three adapters consume the same shared `AuthGate.permission` field — no manifest schema churn.
- examples/authz-demo manifest gains one MCP tool entry (`forecast_tool`) and one gRPC method entry (`forecast_rpc`), both with `auth.permission: { resource: weather, action: read }` inline. Supporting docs/schemas/proto added.
- Smoke tests in `crates/starter-authz/tests/surface_decisions.rs`:
- `rest_mcp_grpc_denies_share_audit_trail_distinguishably` — three denies bound to three surfaces produce three distinguishable rows.
- `check_without_surface_scope_leaves_column_null` — pre-7d.2 in-process callers keep working unchanged.
- All `starter-authz` (sqlite) tests pass; `starter-ext-mcp`/`starter-ext-grpc` build and existing tests pass; `starter-authz-demo` checks clean.
- Committed as `3740d1f` on `codeless/authz-phase-7`.

## Next

- Stage 6 (final sweep) per the job spec: full `cargo test --workspace` across both workspaces, clippy/fmt, end-to-end smoke transcript (boot demo, exercise REST + MCP + gRPC, page /v1/authz/decisions and observe three surface labels), update CHANGELOG/SCOPE-EXT.md status, write the exit summary.

## What you need to know

- The starter workspace and `starter-extensions/` are separate cargo workspaces; package selectors must be issued from the matching workspace root.
- `starter-extensions/crates/starter-ext-mcp` and `starter-ext-grpc` now depend on `starter-authz` (`{ workspace = true }`). Workspace `Cargo.toml` already aliases it from `../crates/starter-authz`.
- Surface label is opt-in: the engine writes `NULL` when the check isn't entered via `with_surface(…)`. Existing in-process call sites (background jobs, db_engine cache rebuild, tests) don't need to change.
- `AuthzedToolBinding` denies with `Error::Forbidden` when no Principal is bound on the task. That's the fail-closed default for the stdio MCP transport — stdio is single-user, no Authenticator. If a future use case needs gated stdio MCP, set a Principal explicitly with `starter_mcp::with_principal(p, fut)`.
- gRPC adapter only enforces when BOTH `engine` and `authenticator` are wired AND the entry has `auth.permission`. Missing either disables the gate (zero-overhead path; matches REST when manifest doesn't declare a gate). Consumers must call `ExtensionGrpcService::with_authz(...)` explicitly.
- REST `with_permission` middleware change is source-compatible — only the wrapping around `engine.check` changed.
- The demo's `server.rs` was not modified to wire MCP/gRPC adapters; the demo's REST path still proves end-to-end. Wiring the demo's MCP + gRPC servers (with the new `_with_engine`/`with_authz` constructors) is part of stage 6's end-to-end smoke transcript.

## Open questions

- Should the MCP `AuthzedToolBinding` also honour `auth.require_role` / `auth.require_scope`? Currently MCP only enforces the per-user `permission` field (role/scope on MCP tools is NYI in this stage; the REST adapter handles all three). Not blocking for 7d.2, flagged in the module docs.
- The gRPC adapter currently re-runs `authenticator.verify(token)` on every call; that's identical to REST behaviour but worth revisiting if a high-QPS extension shows up — could cache verified principals keyed by token hash with a TTL.
