## Done

- Extended `starter-ext-spi` manifest with a `RestStreaming` enum (`none` / `sse` / `ndjson`) replacing the old `streaming: bool` on `ContributeRest`; re-exported through `starter-ext-spi::lib` and `starter-ext-sdk::lib`; new parse test added (`rest_streaming_modes_parse`).
- Added `pub fn BuiltinEntry::dispatch_arc()` to `starter-ext-sdk::builtin` so adapter crates can move the dispatch closure onto `spawn_blocking` without touching private state.
- New module `starter-extensions/crates/starter-ext-server/src/rest/` (mod.rs, dispatcher.rs, handler.rs, router.rs, schema.rs, auth.rs):
- `RestDispatcher` trait (`dispatch` + `dispatch_stream`), `BuiltinRestDispatcher` (routes through `BuiltinTable` + `ExtensionRegistry`), `NotWiredDispatcher` default for non-builtin records.
- `rest_router(registry, dispatcher, options)` that walks every `Validated` record and mounts: tools as `POST /tools/<id>` (R13 — one handler, MCP + REST), REST entries at the manifest's `method` + `path` (with optional adapter `path_prefix`).
- Path/method collisions across extensions are detected in a pre-pass and returned as `RestBuildError::Collision { method, path, first, second }`; the diagnostic carries both `<extension>:<contribute_id>` ids.
- Per-entry `AuthGate` applied as outer middleware via `starter-server::auth::{with_role, with_scope}`; unknown role string is a load-time `RestBuildError::UnknownRole`.
- Request body parsed and validated against the manifest's `input_schema` / `request_schema` via a minimal `SchemaCheck` (type + required); schema violation is 400.
- `RestStreaming::Sse` renders the extension's `Stream<Item = Event>` as `text/event-stream` with `KeepAlive(15s)` and an `X-SSE-Retry-Ms: 3000` reconnect-delay header; `RestStreaming::Ndjson` renders newline-delimited `application/x-ndjson`.
- Client disconnect → `CancelDropGuard` on the response body → `CancelHandle::fire()` → `tokio::sync::watch::send(true)` → `ctx.cancel().is_cancelled()` flips inside the builtin handler within a few hundred ms.
- New integration test `starter-extensions/crates/starter-ext-server/tests/rest_routes.rs` (8 tests) covering: tool route mounting, schema rejection, REST GET dispatch, auth gate (`require_role: admin` returning 401 without a principal), path-collision-is-a-load-error, SSE & NDJSON content types and `X-SSE-Retry-Ms`, and the **Streaming-response-cancels-promptly** smoke test asserting cancel within 500ms of client drop.
- Added `http-body-util` + `futures` dev-deps to `starter-ext-server/Cargo.toml`; wired `starter-ext-sdk` (with `builtin` feature) into its dependencies. Bumped the lockfile.
- Committed with the stage-title commit message; `cargo test -p starter-ext-spi -p starter-ext-host -p starter-ext-mcp -p starter-ext-server -p starter-ext-supervisor` all green.

## Next

- (none) — Stage 14 will be picked up in a fresh session.

## What you need to know

- The `streaming` field on `ContributeRest` is now a `RestStreaming` enum, not a bool. Any future manifest writing `streaming: true` will fail to parse; the test suite and SCOPE example don't currently use that shape so nothing else needed updating.
- The REST adapter is wired only for builtin-flavour extensions in this stage. Process and WASM records return `503 Service Unavailable` via `BuiltinRestDispatcher::ensure_builtin`; the dispatcher trait shape stays unchanged when those slices land.
- The post-R13 cancel-on-disconnect path uses `tokio::sync::watch` + the SDK's `Cancel` trait. Extensions that don't poll `ctx.cancel().is_cancelled()` (or `.cancelled().await`) won't observe cancellation — same contract every adapter relies on.
- The non-streaming dispatch path uses `spawn_blocking` so a long sync builtin handler doesn't stall the runtime. The streaming path also uses `spawn_blocking` and exits when either `ctx.cancel()` flips or the `mpsc::Sender<Event>` errors (receiver dropped).
- Running a whole-workspace `cargo build` from `starter-extensions/` still fails on the pre-existing duplicate-`__STARTER_EXT_FLAVOUR_MARKER` issue caused by feature unification across the three `hello-*` examples — not introduced by this stage. Per-crate builds and tests pass.

## Open questions

- (none)
