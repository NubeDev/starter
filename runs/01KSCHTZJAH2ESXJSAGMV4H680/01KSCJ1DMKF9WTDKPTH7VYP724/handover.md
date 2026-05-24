## Done

- Added `crates/starter-store-postgres/migrations/scheduled_flows/0001_init.sql` defining `starter_scheduled_flows` with the SCOPE Phase A columns (ULID PK, tenant_id, flow_id, cron_expr, next_run_at, last_run_at, last_run_status with CHECK, last_run_message, enabled, created_at, created_by), UNIQUE(tenant_id, flow_id), partial index on enabled rows, and two pg_notify triggers on channel `starter_scheduled_flows` — AFTER INSERT, and AFTER UPDATE scoped to `next_run_at` / `enabled` columns
- Exposed `SCHEDULED_FLOWS_MIGRATOR` + `SCHEDULED_FLOWS_MIGRATION_SOURCE` via new `src/scheduled_flows.rs` and re-exported from `src/lib.rs`
- Added `tests/scheduled_flows.rs` (testcontainers-gated, `#[ignore]`) covering notify on INSERT, UPDATE of next_run_at, UPDATE of enabled, silence on bookkeeping-only update, and UNIQUE enforcement — both tests pass against the real container
- Committed as `phase A.2 — scheduled_flows PG migration — feat(starter-store-postgres) scheduled_flows table + notify`

## Next

- Stage 3: phase A.3 — implement `NodeBehavior::invoke` body for `starter-flow-nodes/src/trigger_schedule.rs` (currently a 23-line KIND_ID/descriptor stub) using the `starter-cron::next_fire` helper

## What you need to know

- Migration source is registered unconditionally (no feature gate), mirroring how `starter` is exposed; downstream callers add it via `migrate(pool).with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)`
- The UPDATE trigger is intentionally scoped to `next_run_at, enabled` so the Phase B tick loop's `last_run_*` writes do not generate notify storms — the test asserts that quietness
- Table is named `starter_scheduled_flows` (prefixed) consistent with the SCOPE channel name; rubix consumers will reference this exact identifier
- Worktree path is `/home/user/.codeless/worktrees/job-01KSCHTZJAH2ESXJSAGMV4H680` — `Write` calls must use that prefix, not `/home/user/code/rust-starter`

## Open questions

- (none)
