## Done

- Added `routes` feature deps (`axum`, `http`, `bytes`) and dev-deps (`tower`, `http-body-util`, `http`, `serde_json`) to `crates/starter-prefs/Cargo.toml`.
- Implemented `crates/starter-prefs/src/routes.rs`: `PrefsRoutesState` (Arc<dyn PrefsStore> + Arc<SystemDefaults>), `prefs_router<S>()`, five `#[utoipa::path]` handlers, `PrefsApi` + `openapi()`, public `DEFAULT_WORKSPACE = "@starter/default"`.
- PATCH body parsing distinguishes missing key vs JSON null via `serde_json::Value`; admin guard is inline (`Option<Response>` shape) so no starter-server dep.
- `/v1/units` payload + ETag are computed once via `OnceLock` (FNV-1a hex, strong ETag), `X-Platform-Version` = crate version.
- Added `crates/starter-prefs/tests/routes.rs` with 6 tokio tests covering the four stage-6 contracts (GET-after-PATCH, PATCH null → org → default, /v1/units byte-stable + ETag stable, 401/403 on org routes, 401 on /v1/me).
- `cargo test -p starter-prefs --features sqlite,routes` + `cargo clippy ... -- -D warnings` both green. Commit `b3e0463`.

## Next

- Stage 7 picks up next per the 22-stage plan (likely starter-client-rs methods + starter-cli `prefs` subcommand, or middleware — check WORKFLOW.md).

## What you need to know

- Active-workspace resolution convention introduced here: `principal.extra["active_workspace"]` (string), then `"@starter/default"` sentinel. If a different convention is preferred for stage 7+ (Principal field, separate header), the helper `workspace_for()` is the single place to change.
- `OnceLock` cache for `/v1/units` is process-wide; if a future stage adds dynamic-registry tests they will need to bypass the cache.
- `apply_user_patch` / `apply_org_patch` reject unknown JSON keys with 400; if forward-compat "ignore unknown" is desired later, that's the place.
- Handlers are wired by value (not `State<>` extractor) to match the starter-ui-theme pattern; this keeps the router generic over consumer `AppState` without state-type gymnastics.
- starter-server's `openapi::merge::merge_starter_paths` is still a stub — wiring `starter_prefs::routes::openapi()` into it is a Phase 2/3 concern.

## Open questions

- (none)
