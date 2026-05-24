## Done

- Created `rubix/crates/rubix-store-postgres` crate with `undo_snapshots` migration (`migrations/undo/0001_undo_snapshots.sql`) and `UNDO_SNAPSHOTS_MIGRATION_SOURCE`. Registered in workspace members + alias.
- Wired the new migration source into `rubix-agent::boot::migrations::apply_migrations` chain.
- Added `[undo]` section to `AgentConfig` via new `UndoConfig { max_rows_per_resource = 50, max_age_days = 90 }`.
- Added `rubix-agent::boot::undo_sweep` with `sweep_once` (single CTE+DELETE using row_number per `(tenant_id, resource_kind, resource_id)` OR `created_at < NOW() - $days`) and `spawn_undo_sweep` (boot-tick + 24h tokio interval). Spawned from `main.rs`.
- Integration test `tests/undo_sweep_test.rs` (testcontainers-gated, `#[ignore]`) seeds 100 rows, asserts ≤ 50 remain after sweep, then asserts age-zero sweep clears the table.
- Two commits: `stage A.2: feat(rubix-store-postgres) …` and `stage A.2: feat(rubix-agent) …`.

## Next

- Stage 3 of the job (continuing Phase A bundled-flow `allowed_tools[]` per SCOPE Phase A, item 3) or Phase B (Goal 2 user-admin) per the WORKFLOW order.

## What you need to know

- `rubix-store-postgres` is a new workspace member; its `MigrationSource` is added after the auth/changelog sources so `_sqlx_migrations_undo_snapshots` is its own state table.
- Snapshot schema: ULID PK as TEXT, JSONB body, `CHECK` on the 7 resource kinds, partial index over `superseded_at IS NULL` for the `rubix.undo.last` hot path.
- Sweep cadence (24h) is a const, only the limits are operator-tunable. Boot-tick failures log a warn but do not abort boot.
- The integration test requires Docker; it's `#[ignore]` for the default `cargo test` run, matching the pattern used by other PG tests in the crate.
- `cargo build -p rubix-agent --tests` succeeds; only stale upstream `default-features` warnings remain (pre-existing).

## Open questions

- (none)
