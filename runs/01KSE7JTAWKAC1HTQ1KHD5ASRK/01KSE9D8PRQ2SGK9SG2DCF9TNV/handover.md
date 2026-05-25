## Done

- Added `rubix-agent/src/sdui/` with four trait-impl files: `entity_graph.rs` (RubixEntityGraph + pluggable SystemSlotReader seam), `page_provider.rs` (PgPageProvider over DashboardStore w/ bundled & per-tenant builders), `query_engine.rs` (RubixQueryEngine routing `ch:` / `pg:` / `mem:` prefixes), `handler_registry.rs` (RubixHandlerRegistry::build wrapping every Tool as an ActionFn → ToastAndRefresh / Diagnostics).
- Wired `pub mod sdui;` in `rubix-agent/src/lib.rs` and added `starter-ui-ir`, `starter-ui-bindings`, `starter-sdui-routes` deps in the agent's `Cargo.toml`.
- Sibling integration tests: `tests/sdui_entity_graph_test.rs`, `tests/sdui_page_provider_test.rs`, `tests/sdui_query_engine_test.rs`, `tests/sdui_handler_registry_test.rs`. Plus per-module `#[cfg(test)]` unit tests.
- `cargo test -p rubix-agent` green; committed as `phase B.1 — SDUI host glue four trait impls`.

## Next

- Stage 5: B.2 — boot wiring (`boot/sdui.rs`), the moka page cache, NOTIFY listener, and per-resolve PG slot cache so `flow:<id>` / `user:<id>` start answering through the entity graph (the v1 arms currently return `None`).

## What you need to know

- `starter_spi::tool::ToolDefinition` has `name`, `description`, `input_schema` — there is no `id` field; the handler registry uses `definition().name` as the handler key.
- `starter_ui_bindings::EntityGraph::read_slot` is **synchronous**; rubix's PG-backed kinds need a per-resolve cache populated before the binding evaluator runs (Phase B.2 work, called out in `entity_graph.rs` doc comments).
- `PageProvider::lookup_page` is single-argument; v1 hard-codes `BUNDLED_TENANT` via `PgPageProvider::bundled`. Per-tenant scoping ships with the resolver middleware in Phase B.3 (the `for_tenant` builder is already in place).
- `test_handler_context` is exposed publicly (not feature-gated) so sibling integration tests can build a `HandlerContext` without spinning the HTTP router.
- `RubixQueryEngine` v1 returns deterministic empty pages from `pg:` / `ch:` backends; RSQL → SQL/CH translation is Phase B.4.
- Existing Docker-gated PG integration tests remain `#[ignore]`; the four new sibling tests run without containers.

## Open questions

- The scope doc mentions a `WritePlanAcl` seam and a `MessageCatalogue` impl in the same directory; this stage scoped them out per the stage description (only the four trait impls). Confirm B.2/B.3 picks them up.
- The flow-engine's `read_slot` is keyed on `(flow_id, slot)` `SlotRef`s and only exposes per-run outputs — it is **not** the seam the entity graph reads through. I noted this in `entity_graph.rs` doc; flag if the scope intended a different wiring.
