## Done

- Added `crates/starter-sdui-routes` with `Cargo.toml` (deps: starter-ui-ir, starter-ui-bindings, axum, http, tokio, serde, serde_json, thiserror, tracing, async-trait; dev: tokio test-util, tower, http-body-util).
- Wired the crate into the workspace `members` list and `workspace.dependencies` table.
- `sdui_router(SduiState)` mounts `POST /api/v1/ui/resolve`, `POST /api/v1/ui/action`, `GET /api/v1/ui/table`.
- `SduiState::builder()` requires PageProvider + EntityGraph + HandlerRegistry + QueryEngine; `InMemoryPageProvider` and `InMemoryQueryEngine` ship as reference impls (S-D2).
- `HandlerRegistry` stores boxed `Fn(HandlerContext) -> ActionFuture` closures; `dispatch` wraps every fire in `tokio::time::timeout(MAX_HANDLER_TIMEOUT)` and surfaces a stable `handler_timeout` what-tag on expiry.
- `/resolve` enforces R8 in order (page_state_bytes → tree shape → binding substitution → capability filter → serialised bytes) and returns `{ render, subscriptions }`.
- `/action` returns the discriminated `ActionResponse` union; unknown handler → 404 with a `diagnostics`-shaped body (`code: "handler_not_found"`).
- `/table` enforces `table_rows_per_page` then delegates to `QueryEngine::query`.
- R7 capability handshake lives in `capability.rs` with threat-model docs in-source: `renderer_id` is public, capability filter is vocabulary not auth, unknown ids rewrite to `Component::Dangling`.
- DoS limits live in `limits.rs` as constants + enforcement helpers; `WhatTag` enum carries the seven stable strings (page_state_bytes, render_tree_bytes, tree_nodes, tree_depth, component_types, handler_timeout, table_rows_per_page).
- Integration tests in `tests/limits_413.rs` pin one tag per limit (handler_timeout uses paused tokio clock); `tests/action_not_found.rs` pins the 404 diagnostics shape; `tests/resolve_table.rs` smokes the happy paths.
- All 21 tests pass; full `cargo build --workspace` succeeds; `starter-server` does NOT depend on the new crate.
- D4 row in `DOCS/frontend/sdui/DIVERGENCE.md` extended with the Phase 5 landing note (crate path, four state pieces, the seven what-tags, R7 vocabulary-vs-auth boundary).
- Committed as `e684992` with the stage 7 commit message verbatim from the brief.

## Next

- Stage 8 picks up Phase 6: remaining IR components (chart, sparkline, tree, timeline, markdown, wizard, drawer, rich_text, diff) plus streaming text/markdown via subscription, full R8 enforcement, falsification suite (CRUD + diff + state-board).

## What you need to know

- `Component::Custom` exists in the IR but the capability filter only rewrites it when the client advertised a non-empty `custom_renderers` list — empty list = "trust the server" (matches R7's "capability filter is vocabulary not auth").
- `enforce_tree_shape` walks the typed tree; `enforce_json_depth` is also exposed for callers that want to depth-check raw JSON before deserialising (defensive against deep-recursion crashes). The wired-in `/resolve` only runs `enforce_tree_shape`, which suffices because the body has already deserialised by then; if a future caller wants to pre-check raw bytes, the helper is there.
- `MAX_COMPONENT_TYPES` is enforced but no integration test forces it (the IR currently has <60 types so triggering it would require synthetic effort); the wire string is pinned by a unit test on `WhatTag::ComponentTypes.as_str()`. SCOPE R8 row already flags this as "inherited / unmeasured."
- `tokio::time::pause/advance` requires the `test-util` feature; added to dev-deps only.
- The `seed_page`-style `PageProvider` here is async (real DBs), distinct from `starter-ui-builder::PageStore` which stays sync per D7. They serve different sides: PageStore writes at boot, PageProvider reads at request time.

## Open questions

- (none)
