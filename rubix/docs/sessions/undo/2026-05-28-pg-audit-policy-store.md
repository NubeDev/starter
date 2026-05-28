# Session 2026-05-28 — `PgAuditPolicyStore`

## Slice

Shipped the Postgres-backed `AuditPolicyStore` so the
`rubix.audit.policy.list` / `rubix.audit.policy.set` verbs
persist across restarts when `pg_pool` is available. Mirrors
the existing `PgDashboardStore` pattern: trait in `rubix-spi`,
in-memory fake stays in `rubix-tools`, Pg impl lives in
`rubix-store-postgres`, registry selects via
`match pg_pool { Some(p) => Pg..., None => InMemory... }`.

This is the first of the four "Pg backing store" slices the
session brief flagged as the smallest available work
(`PgAuditPolicyStore` → `PgRubixTenantStore` → `PgUserAdminStore`
→ `PgTeamAdminStore`). Chosen first because the underlying
table is the smallest (three columns, no joins, no membership
map) and the audit-policy verbs are the shortest verb pair on
the surface — short Pg impl, short verb, easy to validate.

## Decisions

### Trait placement — moved to `rubix-spi::audit`

The audit-policy trait + row originally lived in
`rubix-tools::audit::store`. The module doc already declared
the production path as `rubix_store_postgres::PgAuditPolicyStore`
but no impl had been written. Two ways to satisfy the dep
direction:

1. Add `rubix-tools` as a dependency of `rubix-store-postgres`.
2. Move the trait + row type to `rubix-spi` (the contracts hub),
   matching the `DashboardStore` pattern.

Picked option 2. Workspace `Cargo.toml` documents the layering
explicitly:

```text
# Six crates: rubix-spi → starter-spi; rubix-tools, rubix-skills,
# rubix-flows, rubix-client all → rubix-spi; rubix-agent binds them.
```

`store-postgres` is a sibling of `tools` and `client`, not a
dependency leaf of `tools`. Option 1 would have inverted that
arrow and pulled the entire `tools` compile graph (descriptors,
every goal module, every Reversible) into the thin DB layer.

The move shipped as:

- `rubix-spi/src/audit/{mod.rs, store.rs}` — contract surface
  (trait + `AuditPolicyRow` + `AUDIT_POLICY_KIND`). Zero
  `sqlx` / `tokio` deps; pure `async_trait` + `serde`.
- `rubix-tools/src/audit/store.rs` keeps `InMemoryAuditPolicyStore`,
  `AuditPolicyReversible`, the parse helper, and the in-process
  tests. Top of the file `pub use`s the moved items so existing
  verb imports (`use crate::audit::store::{AuditPolicyRow,
  AuditPolicyStore};`) keep compiling — no churn in
  `policy_list.rs` / `policy_set.rs`.

### Timestamp round-trip — chrono bridge

`AuditPolicyRow.updated_at_ms: i64` vs Postgres `TIMESTAMPTZ`.
Decode via `chrono::DateTime<Utc>` and project to
`timestamp_millis()` on the way out. The undo path (`put`)
converts back via `DateTime::<Utc>::from_timestamp_millis`,
rejecting out-of-range values with `Error::Invalid` rather than
letting `sqlx` silently truncate. Postgres TIMESTAMPTZ has
microsecond resolution so the ms round-trip is exact.

### Concurrency — `FOR UPDATE` on the no-op fast path

`upsert` must satisfy a strict contract:

- Same `(kind, max_age_days)` is a no-op → return identical
  prior/new without touching `updated_at`.
- Different `max_age_days` → bump `updated_at` via `NOW()`.

A naive `SELECT` + branched `UPDATE` races: two writers both see
`prior.max_age_days = Some(30)`, both propose `Some(30)`, both
short-circuit — but if one of them had proposed `Some(60)` the
SELECT would have missed the in-flight write. `FOR UPDATE` on
the prior-row SELECT inside a transaction serialises concurrent
writers for the same kind. The empty no-op transaction still
commits to release the lock. The verb's §3.4 redo-stack-clear
contract relies on this distinction — a missed no-op would
record a phantom `ChangeDraft` and wipe operator redo state.

## Wiring

- `rubix-spi/src/lib.rs` — `pub mod audit;` (alphabetical
  before `dashboard`).
- `rubix-store-postgres/src/lib.rs` — `pub mod audit;` and
  `pub use audit::PgAuditPolicyStore;`.
- `rubix-agent/src/registry.rs` — swap the unconditional
  in-memory wiring for the same `match pg_pool` shape
  `PgDashboardStore` uses.

No verb code, no DTO, no descriptor changes. The slice is pure
substrate; the existing verbs picked up persistence by
construction.

## Tests

- `rubix-tools::audit::store::tests` (in-tree) — five existing
  tests still pass against the in-memory store after the trait
  moved; the `pub use` shim guarantees identical type paths.
- `rubix-agent/tests/audit_policy_pg_test.rs` (new) — six
  scenarios behind `#[ignore]` (testcontainers Postgres, parity
  with `dashboards_definitions_test`):
  1. Seeded floor rows surface in stable order with
     `max_age_days = None`.
  2. Fresh upsert + get round-trip echoes byte-exact.
  3. No-op upsert preserves `updated_at_ms` across a real clock
     tick (contract for verb idempotency).
  4. Changed curve advances `updated_at_ms` strictly.
  5. `put` restores a snapshot verbatim incl.
     `updated_at_ms` (undo path).
  6. `delete` is idempotent on missing rows; only the targeted
     row is removed.

## Gates

- `cargo build --workspace` clean (one full rebuild after
  `cargo clean` to recover disk; ~150 GB still on partition).
- `cargo test -p rubix-tools --lib` → 276 / 276 green.
- `cargo clippy -p rubix-tools -p rubix-spi -p rubix-agent
  -p rubix-store-postgres --lib --tests` clean for my files
  (pre-existing `items_after_test_module` warning in
  `warehouse_write.rs` left alone — off-slice).
- `cargo test -p rubix-agent --test undo_dispatch_test
  --test goal_2_user_admin_test --test admin_registry_test`
  → 3 / 3, 2 / 2, 9 / 9 green.
- `cargo test -p rubix-agent --test audit_policy_pg_test
  --no-run` compiles (no Docker in this sandbox to run the
  ignored cases).

## Off-limits respected

Admin-zone files (`admin/`, `boot/auth.rs`, `tail_listen.rs`)
last touched 2026-05-28 10:28 — ~11 min before this slice
started. Hot. No edits in the zone. The slice touches only
substrate (`rubix-spi`, `rubix-store-postgres`,
`rubix-tools/audit/store.rs`) and the registry's audit-policy
wiring line.

## Follow-ups

- `PgRubixTenantStore` next (single table, similar shape, less
  contract surface than the user/team variants).
- The `changelog` migration source needs to be applied **before**
  this store's table exists. The test wires both
  `changelog` and `changelog_policy` sources explicitly — the
  agent boot path already does this in the right order via
  `boot/migrations.rs`, but the documentation note belongs in
  the future ops runbook.
- The `policy_set` verb echoes the prior row already (no
  change), so the Pg `upsert` returning `(prior, new)` lines up
  with §3.1 with zero verb changes.
