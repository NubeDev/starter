## Done

- Added `rubix/crates/rubix-agent/src/boot/sdui.rs` with `build_sdui_router(&cfg, pg_pool, ch_client, &tools)` composing `PgPageProvider::bundled`, `RubixEntityGraph::new` (with a zero-value `StaticSystemReader`), `RubixQueryEngine::new`, and `RubixHandlerRegistry::build`, returning the merge-ready `Router` from `starter_sdui_routes::sdui_router`.
- Registered `pub mod sdui;` + `pub use sdui::build_sdui_router;` in `boot/mod.rs`.
- Merged the SDUI router into `main.rs` next to the extensions mount (inside the `Some(dsn)` branch). Cloned `ChClient` into `ch_client_for_sdui` before the tool-registry consumes it, and cloned the `tools` vec where it's passed into `ToolsState::new`.
- `cargo test -p rubix-agent` green (41 lib, plus all SDUI integration tests).
- Committed as `stage 5: phase B.2 — mount sdui_router under /api/v1/ui`.

## Next

- Stage 6 of 16 (next phase per the scope dependency graph in `rubix/docs/scope/dashboards/README.md`).

## What you need to know

- The upstream `sdui_router` already roots routes at `/api/v1/ui/...`, so the wiring uses `.merge(sdui_router)`, **not** `.nest("/api/v1/ui", ...)` as the scope draft suggested. This matches how axum composes the upstream routes.
- The SDUI mount lives inside the `if let Some(dsn) = cfg.database_url` block because `PgPageProvider`/`RubixEntityGraph` require a live pool. Laptop boot (no DSN) still skips SDUI — same posture as extensions.
- `StaticSystemReader::new()` is a placeholder; Phase B.3 swaps it for a tool-registry-backed reader (one-line edit per the file comments).
- The known-but-not-yet-landed `flow_events` router from the rubix-flow-live-tick-demo job will live as an adjacent line in main.rs — rebases trivially.

## Open questions

- (none)
