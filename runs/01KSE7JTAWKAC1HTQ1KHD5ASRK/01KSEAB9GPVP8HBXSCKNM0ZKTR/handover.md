## Done

- Filled DTOs in `rubix/crates/rubix-spi/src/dto/dashboard/{get,list}.rs` (utoipa `ToSchema` request/response + descriptor + `REQUIRED_PERMISSION`); added `get` to `dto/dashboard/mod.rs`.
- Filled tool bodies in `rubix/crates/rubix-tools/src/dashboard/{get,list}.rs` as `DashboardGetTool` / `DashboardListTool`, each dispatching through `Arc<dyn DashboardStore>` (production wires `PgDashboardStore`); added `get` to `dashboard/mod.rs`.
- Added 3 MessageKeys to both EN + ES catalogues in the same commit: `rubix.dashboard.fetched`, `rubix.dashboard.listed`, `rubix.dashboard.get.not_found`.
- Sibling unit tests: 7 new tests under `dashboard::get::tests` + `dashboard::list::tests` covering hit/miss/tenant-isolation, empty store, sort, tag-overlap filter, owner filter.
- `cargo test -p rubix-tools -p rubix-spi` green (all 7 dashboard tests pass; no other regressions).
- Committed as `0bd48c0` — message starts with the stage title `phase C.1 — dashboard.get + dashboard.list reads`.

## Next

- (none — next stage picks up Phase C.2 per `rubix/docs/scope/dashboards/04-tools.md`)

## What you need to know

- `DashboardStore` trait lives in `rubix-spi/src/dashboard/store.rs`; tool bodies hold `Arc<dyn DashboardStore>` so unit tests swap in an in-file `InMemoryStore` (no PG needed). The real wire-up to `PgDashboardStore` happens at agent boot — both tools follow the `FlowListTool` pattern from `flow_ops/list.rs`.
- `GetDashboardResponse` keeps all body fields `Option`-wrapped so the same struct serialises for both hit (`rubix.dashboard.fetched`) and miss (`rubix.dashboard.get.not_found`) without a discriminated union.
- `ListDashboardsRequest` carries `tenant_id` + optional `tags_any` + `owner`; sort is by `page_id` ascending. Per-row `rubix.dashboard:view` authz filtering is deferred to Phase C.2 per scope.
- Catalogue files (`rubix/crates/rubix-spi/catalogues/{en,es}.json`) are embedded via `include_str!`; both must keep matching keys or `rubix_bundle()` parses but consumers may miss translations.

## Open questions

- (none)
