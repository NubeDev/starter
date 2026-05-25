## Done

- Added migration `rubix-store-postgres/migrations/dashboards_definitions/0001_dashboards_definitions.sql` (insert-only revisions, partial active-set index, GIN tag index, `pg_notify` trigger on `rubix_dashboards_definitions`).
- Added `DashboardStore` async trait + `DashboardRevision` / `NewRevision` / `ListFilter` / `DashboardStoreError` value types under `rubix-spi/src/dashboard/` (zero SQL deps; added `async-trait` to rubix-spi Cargo).
- Implemented `PgDashboardStore` in `rubix-store-postgres/src/dashboards/mod.rs` (insert_revision supersedes prior live row inside one tx; get_active / list_active with tenant+owner+tags filter / mark_superseded / history). Added required sqlx features (chrono/json/uuid) + chrono/uuid/serde_json/async-trait/thiserror/rubix-spi deps.
- Exposed `DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE`, `DASHBOARDS_DEFINITIONS_CHANNEL`, `DASHBOARDS_DEFINITIONS_MIGRATOR`, `PgDashboardStore` from the crate root.
- Wired the migration source into `rubix-agent::boot::migrations`, pre-registered the `rubix.dashboard.page` resource kind in `boot::authz` (tenant-scoped, actions view/edit/delete, Ownership::Subject), and added `boot::dashboards_seed` (idempotent seeder mirroring `flows_seed`; re-asserts the resource kind on every insert via `try_register`, ignores `DuplicateResource`; laptop fallback when pool is `None`).
- Added integration test `rubix-agent/tests/dashboards_definitions_test.rs` covering insert / supersede / list (tag filter + tenant scoping) / history / delete. `#[ignore]` like its `flows_definitions` sibling because it needs the testcontainers Postgres.
- `cargo build` + `cargo test -p rubix-spi -p rubix-store-postgres -p rubix-agent` green.
- Committed as `2fb4d58 feat(rubix-store-postgres+rubix-spi) dashboards_definitions + PgDashboardStore + authz registration`.

## Next

- Phase A.2 — host glue (`03-host-glue.md`): rubix-side `PageProvider` impl that reads from `PgDashboardStore`, `EntityGraph` / `QueryEngine` / `HandlerRegistry` impls, and `sdui_router` wiring into the agent. Plus the `rubix-agent::boot::dashboards_notify` listener on `rubix_dashboards_definitions` to invalidate the cache cross-instance.

## What you need to know

- I used the existing flat folder convention `migrations/dashboards_definitions/0001_*.sql` (matches `flows_definitions/0001_flows_definitions.sql`) rather than the literal `<NNNN>_dashboards_definitions/up.sql` shape in the stage prompt — sqlx::migrate! requires numbered .sql files at the source root.
- `dashboards_definitions.tenant_id` is TEXT (not UUID like flows_definitions) so bundled rows can carry the `"system"` sentinel and the column matches the page_id / principal styling already used by SDUI. The `BUNDLED_TENANT` / `BUNDLED_PRINCIPAL` consts live in rubix-spi.
- `boot::dashboards_seed::seed()` looks for bundled pages under `rubix_flows::BUNDLED.get_dir("dashboards")` which does not exist yet — the seeder gracefully returns 0 inserts. Phase A.5 (per scope) moves bundled pages into `rubix/crates/rubix-flows/dashboards/`.
- ResourceSpec is registered by `kind` only (no per-id register); the stage prompt's "register page id as ResourceSpec" was interpreted as ensuring the kind `rubix.dashboard.page` is registered (idempotent via `try_register`). Per-page authz happens via ResourceRef + tenant_scoped predicate.
- `boot::dashboards_seed::seed` is not yet called from `main.rs` — Phase A.2/A.5 will wire it in alongside the page-resolver. The function and its docs are in place.

## Open questions

- The stage spec literal "rubix-spi/src/dashboard/store.rs" placed the trait next to but not under `dto/dashboard/` — matches the layout I chose. Confirm at review.
- bundled pages directory (Phase A.5) — should it live under `rubix/crates/rubix-flows/dashboards/` (re-using `rubix_flows::BUNDLED`) or a dedicated `rubix-flows/src/dashboards.rs` with its own `include_dir!`? Current seed assumes the former.
