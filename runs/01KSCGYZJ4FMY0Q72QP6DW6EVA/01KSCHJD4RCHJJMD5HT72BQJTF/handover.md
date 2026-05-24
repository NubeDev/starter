## Done

- Added `utoipa` workspace dep to `rubix/crates/rubix-agent/Cargo.toml`
- Added `#[utoipa::path]` to `health::healthz` (GET /healthz, tag system) and `routes::tools::dispatch` (POST /api/v1/tools/{tool_id}, tag system); promoted both to `pub(crate)`
- New `rubix/crates/rubix-agent/src/openapi.rs` exposing `pub fn rubix_openapi() -> utoipa::openapi::OpenApi` via `#[derive(OpenApi)] RubixApi`, with info+servers+paths+9 per-goal tags
- New `rubix/crates/rubix-agent/src/routes/openapi_doc.rs` mirroring starter-server's `GET /openapi.json` (unauthenticated)
- Wired into `main.rs`: captures doc at boot, merges openapi-doc router alongside `/healthz` + `/api/v1/mcp` (outside the auth/authz sandwich)
- Integration test `tests/openapi_test.rs` (3 tests): doc parses as JSON, exactly 9 tags including all 9 expected names, canary paths `/healthz` and `/api/v1/tools/{tool_id}` both present
- `cargo test -p rubix-agent --lib` → 19 passed; `cargo test -p rubix-agent --test openapi_test` → 3 passed
- Committed as `d628239` on `codeless/rubix-client-ts`

## Next

- Phase B.3+ work per SCOPE: `rubix/openapi.json` snapshot file + `rubix/scripts/snapshot-openapi.sh`, then Phase C (`@nube/rubix-client-ts` package), then Phase D (drift CI + docs + PR)

## What you need to know

- `dispatch` and `healthz` are now `pub(crate)` — earlier they were private. The `#[utoipa::path]` macro generates a sibling `__path_<fn>` struct whose visibility matches the fn, and `paths(crate::routes::tools::dispatch, crate::health::healthz)` in `openapi.rs` requires at least crate-visible access
- The openapi-doc router is merged unauthenticated by design (matches starter-server precedent + the codegen pipeline contract noted in SCOPE Phase B)
- Tags-per-goal are declared in the document even though only `system` is referenced by a concrete route today; future per-verb handlers (when goals stop dispatching through the single tools handler) slot into `paths(...)`
- The two new unit tests under `openapi::tests` are intentional duplicates of the integration tests' assertions — they catch regressions even when `tests/openapi_test.rs` is not invoked

## Open questions

- (none)
