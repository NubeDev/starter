## Done

- Added `AuthGate.permission: Option<PermissionGate { resource, action }>` to `starter-ext-spi/manifest.rs` (deny_unknown_fields, re-exported).
- Added `with_permission_owned(router, String, String)` to `starter-authz` for manifest-sourced (non-`'static`) kind/action strings; exported from lib.
- Extended `starter-ext-server`'s REST adapter:
- `RestRouterOptions::resource_registry: Option<Arc<dyn ResourceRegistry>>`.
- `RestBuildError::UnknownResource { entry, resource }` symmetric with `UnknownRole`.
- `apply_gate` validates `permission.resource` via `registry.lookup` and wraps with `with_permission_owned`. Layer order documented in code: `with_role (outer) → with_scope → with_permission (inner) → handler` with the audit-consequence rationale.
- Demo updated:
- `examples/authz-demo/extensions/com.acme.weather/block.yaml` declares `auth.permission` inline for both routes.
- `examples/authz-demo/src/weather.rs` reduced to a `BuiltinTable` and a docstring describing the pre-Phase-7d hand-mount (witness "this is what the adapter does for you now").
- `examples/authz-demo/src/server.rs` mounts `/weather/*` via `rest_router` with the `ResourceRegistry` passed through; layer-order + audit-consequence comment is inline at the wiring point.
- Smoke tests in `starter-extensions/crates/starter-ext-server/tests/permission_routes.rs`: `per_entry_permission_applied`, `unknown_resource_is_build_error`, `role_and_permission_compose_correctly` — all pass.
- All targeted crates build clean; pre-existing workspace-wide `__STARTER_EXT_FLAVOUR_MARKER` symbol conflict is unrelated (reproduces on HEAD with no changes).
- Committed as `stage 4 (slice 7d) — REST adapter: AuthGate.permission`.

## Next

- Stage 5 (slice 7d.2) — MCP and gRPC adapter parity: consume the same `AuthGate.permission` field in `starter-ext-mcp` and `starter-ext-grpc`. The manifest shape is already shared; only the two adapter crates need to wire `with_permission_owned` (or their transport's equivalent) and surface `UnknownResource`.

## What you need to know

- `starter-extensions` is a sibling cargo workspace; it now lists `starter-authz` (path = ../crates/starter-authz) as a workspace dependency. Mirrors the existing `starter-server` / `starter-mcp` path-deps.
- `apply_gate`'s layer order is innermost-added-first because `axum::Router::layer` wraps the existing router (most recent `.layer` becomes outermost). Don't "fix" it to put `with_role` inner — the audit asymmetry is intentional per the SCOPE.
- If a manifest declares `auth.permission` but the host passes `RestRouterOptions { resource_registry: None, .. }`, the adapter returns `UnknownResource` for that entry. This is deliberate: without a registry the adapter can't verify the kind, so it refuses to mount.
- `with_permission_owned` is a new public API on `starter-authz`; the original `with_permission(&'static str, &'static str)` is untouched for back-compat with Phase 1–6 callers.

## Open questions

- (none)
