## Done

- Added `rubix/crates/rubix-store-postgres/migrations/flows_definitions/0001_flows_definitions.sql` with SCOPE Phase D shape (ULID PK, tenant_id, flow_id, revision_id, body_yaml, created_at, created_by, superseded_at, UNIQUE(tenant_id, flow_id, revision_id)) plus the AFTER INSERT/UPDATE trigger that fires `pg_notify('rubix_flows_definitions', ...)`.
- Exposed `FLOWS_DEFINITIONS_MIGRATION_SOURCE`, `FLOWS_DEFINITIONS_MIGRATOR`, `FLOWS_DEFINITIONS_CHANNEL` from `rubix-store-postgres`.
- Wired the new source into `rubix-agent::boot::migrations` so it applies alongside `undo_snapshots`.
- Added `rubix-agent::boot::flows_seed` (seed + load helper using `SYSTEM_TENANT = Uuid::nil()`) and refactored `boot::mcp::register::build_flow_registry` to take an optional `Pool`. With a pool it seeds bundled YAMLs on miss then loads every `superseded_at IS NULL` row; without a pool it falls back to `rubix_flows::load_all()`.
- Added `rubix-agent::boot::flow_notify` (~170 lines incl. doc): `PgListener` on `rubix_flows_definitions`, re-reads body from PG, parses → triple, hands to a `ReloadFn` hook.
- Updated `main.rs` to share a PG pool with `build_mcp_surface` and spawn `flow_notify` with a log-only reload stub (the real `FlowRegistry::reload` wiring is intentionally a future stage's responsibility).
- Updated `rubix_admin mcp serve` and `build_tool_registry`/`build_mcp_surface` signatures.
- Integration test `tests/flow_definitions_seed_test.rs` asserts first boot inserts one row per bundled YAML, second boot inserts zero, and revision_id round-trips losslessly. Test is `#[ignore]`d behind the testcontainers Postgres gate (same convention as `undo_sweep_test.rs`).
- Two commits as required: `stage 12: phase D.1 — flows_definitions migration` (rubix-store-postgres) and `stage 12: phase D.1 — FlowRegistry refactor + flow_notify listener` (rubix-agent).
- `cargo build -p rubix-agent --all-targets` clean; `cargo test -p rubix-agent --lib` 17 passed.

## Next

- Stage 13 will pick up the next phase per the job goal — likely the goal-3 flow-programmer verbs that *produce* `flows_definitions` writes (and at that point can swap the `flow_notify` stub for a real `FlowRegistry` re-register call).

## What you need to know

- The stage spec said "calling `FlowRegistry::reload(flow_id, body)` on receipt"; `starter-flow-surfaces::FlowRegistry` has no `reload` method (revisions are immutable, `register` returns `DuplicateRevision` on collision). I modelled the listener as accepting a `ReloadFn` hook so a later stage that adds the real re-register call (under a new revision id, which the notify payload already carries) can drop in without changing the channel/payload contract. Current `main.rs` hook just logs.
- Bundled rows are seeded under the all-zero `tenant_id` + `created_by` sentinel, same convention as `undo_snapshots`.
- The notify payload includes `op`, `id`, `tenant_id`, `flow_id`, `revision_id`, `superseded_at`; the listener filters `superseded_at IS NOT NULL` events and re-reads body from PG (NOTIFY payload is 8 kB-capped).
- `build_flow_registry`, `build_tool_registry`, `build_mcp_surface` all gained an `Option<Pool>` parameter; only call sites are `main.rs`, `bin/rubix_admin/mcp/serve.rs` (passes `None`), and tests (none currently call these). Stage tests for goals 2/4 continue to work through other paths.
- Added deps to `rubix-agent/Cargo.toml`: `uuid` (workspace), `include_dir = "0.7"`, `futures` (workspace), plus `uuid` + `chrono` features on `sqlx`.
- ULID column is filled with a 26-char slice of a UUIDv4 simple-string — uniqueness only, not strict time-sortability. If strict ULID monotonicity is needed later, swap `flows_seed::ulid_text` for a real ULID crate.

## Open questions

- Should the integration test be unmarked from `#[ignore]` once the codeless CI gains a Postgres testcontainers job, or stay opt-in like the sibling undo-sweep test? Following existing convention I left it ignored.
- The `FlowRegistry::reload` shape is not yet defined upstream — the goal-3 stage will need to decide whether to add `register` under a new revision (immutable-history) or introduce a true mutable `reload`. Flagging here so the next session picks an explicit path.
