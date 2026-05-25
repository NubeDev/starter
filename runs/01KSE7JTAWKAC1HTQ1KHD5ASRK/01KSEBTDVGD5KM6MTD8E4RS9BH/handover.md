## Done

- Filled `rubix-spi/src/dto/dashboard/page_set.rs` with `PageSetRequest` / `PageSetResponse` (utoipa ToSchema) + static `DESCRIPTOR`.
- Filled `rubix-tools/src/dashboard/page_set.rs` `DashboardPageSetTool` — funnels one slot write through `starter_flow_spi::graph::GraphStore::write_slot` (R2 chokepoint), coerces JSON→`SlotValue`, validates `NodeId` grammar and non-empty slot, emits `rubix.dashboard.page_set.applied`.
- Added `starter-flow-spi` to `rubix-tools` `[dependencies]` (was dev-only).
- Added `rubix.dashboard.page_set.applied` to en + es catalogues, same commit.
- Sibling tests: round-trip write/read, SlotChanged event emitted (chokepoint proof), coercion variants, invalid node_id rejected, empty slot rejected, idempotent re-write.
- `cargo test -p rubix-tools -p rubix-spi` green — 115 lib tests (was 109).
- Committed as `phase C.4 — dashboard.page_set runtime slot write` (71e654a).

## Next

- Stage 11 picks up the next phase per `rubix/docs/scope/dashboards/README.md`.

## What you need to know

- Deliberately NOT `Reversible` — no `ReversibleTool` impl, no `DashboardSnapshot`. Per SCOPE OQ-5 the revert path is "write the prior value back". Document this in the dashboards design doc when phase C is promoted.
- Tool ctor signature is `DashboardPageSetTool::new(graph: Arc<dyn GraphStore>)` — bootstrap wiring (mcp/register or similar) needs to pass the same `GraphStore` handle the propagator uses, otherwise audit/replay diverge.
- `NodeId` grammar requires a dot; `dashboard.<slug>` satisfies it. A flat id like `thermostat` is rejected with `Error::Invalid`.
- JSON coercion: numbers prefer `Int` then `Float`; objects/arrays land in `SlotValue::Json` rather than being refused.
- Doc comment in `04-tools.md` still describes the *old* page_set semantics (ComponentTree write, Reversible) — promotion to design docs should reconcile against the new runtime-slot semantics this stage shipped.

## Open questions

- Authz: verb body does not yet call `with_permission_owned("rubix.dashboard.edit", ...)` — left to the dispatch middleware sandwich; confirm that's where stages C.1/C.2/C.3 placed it (they did).
- Change-log: `page_set` is non-Reversible so no `change_for`; if a future audit policy needs a per-call row, that lives in the middleware, not the tool.
