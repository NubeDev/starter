# SQLite → Postgres migration (rubix)

> **Tier:** plan, not system-as-it-is. Lives in `docs/scope/` per
> [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md). Source code must
> not reference this file — once each block lands, the design
> moves into `docs/design/<area>/README.md` and code links there.

## TL;DR

rubix already runs on Postgres for everything that matters
(sessions, authz, audit, changelog, flow definitions, runs).
**One** SQLite file remains in the production binary:

- `~/.rubix/node_state.db` — `SqliteNodeStateStore`, wired in
  [`rubix-agent/src/boot/flow_runtime.rs`](../../crates/rubix-agent/src/boot/flow_runtime.rs).

That is the only thing this plan removes from the rubix prod
path. After this lands, rubix runs against a single Postgres
database. SQLite stays in the repo as the upstream `starter-*`
reference impl and as an ephemeral test fixture; rubix simply
stops wiring it.

## Why remove it

- **Two sources of truth.** Flow definitions, runs, agent
  sessions, and changelog all live in PG. Node runtime state
  (counter nodes, scratch state for stateful nodes) lives in a
  separate SQLite file on local disk. A flow can therefore be
  partially recovered after a crash: the run row is in PG,
  the per-node counters are on a different volume.
- **No horizontal scale.** `~/.rubix/node_state.db` is bound to
  one host. The moment rubix-agent runs as more than one
  replica, two replicas see two different node states for the
  same flow run.
- **Two backup stories.** Operators already snapshot Postgres.
  Adding a per-host SQLite file means a second backup, a second
  restore drill, and a per-host disk-failure mode that does not
  apply to anything else in the stack.
- **Two migration toolchains.** `starter-store-sqlite::migrate`
  + `FLOW_MIGRATION_SOURCE` runs alongside `sqlx::migrate!`
  against PG. One engine = one migration path.
- **No real upside.** The original justification ("a laptop boot
  without Postgres still has a working seam") is already covered
  by the in-memory fallback (`InMemoryNodeStateStore`). A
  developer without PG keeps the volatile fallback; an operator
  with PG gets durable state in the same DB as everything else.
  SQLite-as-middle-tier carries the cost of durability without
  the benefit of consolidation.

## What is actually wired to SQLite in rubix today

Audited 2026-05-25 against `git rev-parse HEAD` (record the SHA
in the implementation PR description so a future reader can
re-run the audit). Production code (non-test) under
`rubix/crates/`:

| Site | What | Impl |
|---|---|---|
| [`rubix-agent/src/boot/flow_runtime.rs`](../../crates/rubix-agent/src/boot/flow_runtime.rs) | Node runtime state (`NodeStateStore`) | `SqliteNodeStateStore` over `~/.rubix/node_state.db` |
| [`rubix-agent/src/boot/config.rs`](../../crates/rubix-agent/src/boot/config.rs) | `state_db_path` default | `~/.rubix/node_state.db` |
| [`rubix-agent/src/middleware/changelog.rs`](../../crates/rubix-agent/src/middleware/changelog.rs) | Doc comment only — recorder seam is generic; prod wires PG | n/a |

That's the entire production surface. Everything else
(`flows_definitions`, undo snapshots, dashboards, sessions,
authz, audit, changelog) is already PG via
`starter-store-postgres` + `rubix-store-postgres`.

Test files (`rubix-agent/tests/*`) use ephemeral SQLite via
`starter-store-sqlite::testing::ephemeral` for fixture setup of
upstream SPIs that have both impls. Those are out of scope for
this plan — they exercise the SPI contract, not the wiring.

## What needs to exist upstream first

`starter-store-postgres` already ships `PgFlowStore`,
`PgRunStore`, `PgAgentSessionStore`, `PgSessionStore`. It does
**not** yet ship a `PgNodeStateStore`. The SPI lives in
[`starter-flow-spi/src/state.rs`](../../../crates/starter-flow-spi/src/state.rs);
the SQLite reference impl lives in
[`starter-store-sqlite/src/flow/node_state.rs`](../../../crates/starter-store-sqlite/src/flow/node_state.rs).

**Prereq (upstream PR, lands in `starter-store-postgres`):**

1. New module `starter-store-postgres/src/flow/node_state.rs`
   exporting `PgNodeStateStore` that implements
   `starter_flow_spi::state::NodeStateStore`.
2. Mirror the SQLite schema as a PG migration in
   `starter-store-postgres/migrations/flow/` — the SQLite columns
   map cleanly to PG types (TEXT id, JSONB body, TIMESTAMPTZ).
   Use the same `(flow_id, node_id, slot)` natural key.
3. Re-use the existing JSON-envelope chokepoint pattern from
   [`starter-store-sqlite/src/flow/schema.rs`](../../../crates/starter-store-sqlite/src/flow/schema.rs)
   — `serde_json::Value` round-trip, no schema-aware columns,
   `FlowError::Backend` on serialize/deserialize failures.
4. Contract tests: extend the existing
   `tests/node_state_*` parity tests so the same suite runs
   against both impls (mirrors how `FlowStore` is tested today).

This upstream PR has no rubix dependency and unblocks any
other downstream that wants the same consolidation.

## Migration plan (rubix-side, after the upstream PR)

### Block 1 — wire `PgNodeStateStore` as an option

- In [`rubix-agent/src/boot/flow_runtime.rs`](../../crates/rubix-agent/src/boot/flow_runtime.rs),
  extend the existing picker so when a PG pool is available we
  construct `PgNodeStateStore` instead of opening a SQLite pool.
  `build_flow_runtime` now takes `Option<PgPool>` (the shared
  pool already opened in [`rubix-agent/src/main.rs`](../../crates/rubix-agent/src/main.rs)
  for the MCP `flows_definitions` seed/load); the runtime no
  longer opens its own connection pool from the DSN.
- Keep `InMemoryNodeStateStore` as the laptop fallback when no
  DB URL is configured.
- **Delete** the `SqliteConnectOptions` / `SqlitePoolOptions` /
  `SqlitePool::from_sqlx` branch and the
  `FLOW_MIGRATION_SOURCE` apply call.
- **In-flight state at upgrade time.** This block changes the
  picker; on first boot of the new binary the PG `node_state`
  table is empty while `~/.rubix/node_state.db` still holds the
  surviving state. Pick **one** of these and write it into the
  code in this same block — operators must not silently lose
  in-flight counters:
  1. *Hard pre-upgrade gate.* `build_flow_runtime` refuses to
     boot when `~/.rubix/node_state.db` exists and the PG
     `node_state` table is empty, with an error pointing at the
     Block 3 migration script.
  2. *Boot-time one-shot copy.* `build_flow_runtime` performs an
     idempotent SQLite→PG copy when both sources are present,
     then renames the SQLite file to `*.migrated` so subsequent
     boots short-circuit.
  Recommendation: option 2, because it removes a manual step;
  option 1 is acceptable for operators who prefer explicit
  control. Pick one in the implementation PR and document the
  choice in `CHANGELOG.md`.
- In [`rubix-agent/src/boot/config.rs`](../../crates/rubix-agent/src/boot/config.rs),
  remove `state_db_path` from `FlowRuntimeConfig` and its default
  (`~/.rubix/node_state.db`). The PG pool URL is the only knob.
- Update the boot logs in `flow_runtime.rs`:
  - `"NodeStateStore: Postgres (durable)"` when PG pool present.
  - `"NodeStateStore: in-memory (volatile) — set RUBIX_DATABASE_URL for durability"` otherwise.

Exit criterion (mechanical):
```
grep -R 'use starter_store_sqlite\|SqliteNodeStateStore\|state_db_path' \
    rubix/crates/rubix-agent/src/
```
returns no hits.

### Block 2 — drop the SQLite dep from rubix-agent

- Remove `starter-store-sqlite` from
  [`rubix-agent/Cargo.toml`](../../crates/rubix-agent/Cargo.toml)
  `[dependencies]` (the `features = ["flow"]` line).
- **Keep** the `"sqlite"` entry in the `sqlx` feature list on the
  same file. `[dev-dependencies]` still pulls
  `starter-store-sqlite` with `features = ["testing", "flow"]`
  for hermetic SPI-contract tests, and that transitively needs
  the sqlx sqlite driver. Drop it only if/when the dev-dep also
  goes.
- Audit `[dev-dependencies]`: the ephemeral-SQLite test fixtures
  decide on a per-test basis. Tests that exercise a
  starter-level SPI may stay on SQLite (faster, hermetic). Tests
  that exercise rubix wiring should switch to the same
  PG-fixture pattern used in `goal_3_flow_programmer_test.rs`.
- Run `cargo check -p rubix-agent` and follow the compile errors.
  Expected: zero, because the upstream PR already wired
  `PgNodeStateStore` behind the same SPI trait object.
- Add a CHANGELOG entry under the next rubix release flagging
  the behaviour change for operators who today have both
  `RUBIX_DATABASE_URL` and the default `state_db_path` set:
  before this PR they were on durable SQLite for node state;
  after it, node state moves into PG via the Block 1
  copy/gate. Reference the Block 3 script.

### Block 3 — operator migration (one-shot script)

- Ship `rubix/scripts/migrate-node-state-to-pg.sh` that opens the
  old `~/.rubix/node_state.db` with `sqlite3`, dumps each row,
  and loads it into the new PG `node_state` table.
- **Idempotency pattern (must be specified).** `COPY FROM` does
  not honour `ON CONFLICT` and will fail the second run on the
  unique index. Use the staging-table pattern instead:
  ```sql
  CREATE TEMP TABLE node_state_staging (LIKE node_state INCLUDING DEFAULTS);
  COPY node_state_staging FROM STDIN ...;
  INSERT INTO node_state (flow_id, node_id, slot, body, updated_at)
  SELECT flow_id, node_id, slot, body, updated_at FROM node_state_staging
  ON CONFLICT (flow_id, node_id, slot) DO NOTHING;
  ```
  **Semantics: first-writer-wins (`DO NOTHING`).** Rationale:
  if the agent has booted against PG between script runs and
  written newer node state, the script must not stomp it. An
  operator who wants the SQLite snapshot to override must
  truncate `node_state` first; the script prints a warning to
  that effect when it detects existing rows.
- Operator note in [`rubix/CHANGELOG.md`](../../CHANGELOG.md)
  flagging this as a one-time step at upgrade boundary. Old
  `~/.rubix/node_state.db` is left in place (not deleted) so a
  rollback is trivial.
- If Block 1 picked option 2 (boot-time auto-copy), this script
  is the *escape-hatch* for operators on option 1 or for
  recovery after a failed auto-copy; it stays shipped either
  way.

### Block 4 — delete the dead path

After one release where operators have run the migration script:

- Remove `rubix/scripts/migrate-node-state-to-pg.sh`.
- Remove any `~/.rubix/node_state.db` references from docs.
- Closeout: this scope doc collapses into a one-line entry in
  `docs/design/flow-runtime/README.md` under "history" and is
  deleted from `docs/scope/`.

## What stays on SQLite (and why that's fine)

- **Upstream `starter-store-sqlite` reference impl.** Stays in
  the workspace. Other starter consumers (the `minimal` example,
  laptop-class deployments without PG) keep using it. Removing
  rubix's dep is not the same as deleting the crate.
- **Test fixtures.** Ephemeral in-memory SQLite is fast and
  hermetic; tests that don't exercise PG-specific behaviour can
  keep using it. Per-test decision, not a blanket sweep.
- **Anywhere a future "embeddable rubix" target appears.** None
  exists today; if one does, it picks the SQLite impl back up
  through the same SPI seam — that's what the seam is for.

## Non-goals

- Removing `starter-store-sqlite` from the workspace.
- Touching the bundled-YAML flow loader (`rubix-flows`) — it
  already feeds PG via `flows_definitions`.
- Changing the SPI surface. The whole point of the SPI is that
  this is a wiring change, not a contract change.
- `LISTEN`/`NOTIFY` on `node_state` writes for cross-replica
  live updates: runtime state is per-run and the run pump
  already coordinates within a single agent.

## Open questions

- Should `PgNodeStateStore` use one row per `(flow_id, node_id,
  slot)` like the SQLite impl, or one row per node with a JSONB
  slot map? Defer to the upstream PR; rubix doesn't care which
  shape lands as long as the trait object behaves the same.
- Retention: PG accumulates rows where SQLite was per-host
  rotation. Likely fine (node-state rows are small and tied to
  finite runs) but worth a follow-up if it grows unbounded.

## Cross-refs

- [`docs/scope/README.md`](./README.md) — scope tier definition.
- [`crates/starter-flow-spi/src/state.rs`](../../../crates/starter-flow-spi/src/state.rs) — the `NodeStateStore` trait.
- [`crates/starter-store-sqlite/src/flow/node_state.rs`](../../../crates/starter-store-sqlite/src/flow/node_state.rs) — current impl to mirror.
- `crates/starter-store-postgres/src/flow/node_state.rs` *(to be created in the upstream prereq PR)* — where `PgNodeStateStore` will land.
- [`rubix/crates/rubix-store-postgres/migrations/flows_definitions/0001_flows_definitions.sql`](../../crates/rubix-store-postgres/migrations/flows_definitions/0001_flows_definitions.sql) — pattern to follow for the new PG migration.
