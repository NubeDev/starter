# 2026-05-28 — Undo / redo: proposal §3 landed, prod-wired, metadata-safe

Implementing
[`rubix/docs/proposal/flow-storage-and-undo-redo.md`](../../proposal/flow-storage-and-undo-redo.md)
sections §3.0, §3.4, §3.5, §3.6, plus the prod boot wiring the
proposal itself flagged as missing. Single peer-reviewed v2 proposal,
PG-only path (per user direction: "just focus on PG, rockstar
undo/redo system").

## Scope of this session

Closed all proposal §3 work that can land without a separate
audit-log proposal. The pieces that remain out are explicitly
deferred in the proposal text:

| Proposal § | Status this session | Notes |
|---|---|---|
| §3.0 — Snapshot vs patch policy | ✅ landed | doc-comment + each impl tagged |
| §3.1 — Dashboard rename / re-tag undo | ✅ landed | regression test + DTO change |
| §3.2 — Per-node undo inside a flow | ⏸ deferred until Phase B | per proposal |
| §3.3 — User role / prefs Reversible | ⏸ deferred | needs audit-log proposal first (§3.3 explicit) |
| §3.4 — Redo across processes | ✅ landed | `PgUndoCursor` + epoch CAS + `rubix.undo.redo` verb |
| §3.5 — Per-kind retention policy | ✅ landed | `undo_kind_policy` table + sweep JOIN |
| §3.6 — Drop `tenant` Reversible | ✅ landed | CHECK migration |
| Prod boot wiring | ✅ landed | `UndoSubstrate` constructed from PG pool in `main.rs` |

## What landed

### §3.0 — Snapshot vs patch policy doctrine

[`crates/starter-spi/src/changelog/reversible.rs`](../../../../crates/starter-spi/src/changelog/reversible.rs)
— `Reversible` trait now carries a decision matrix in its rustdoc:
when to store the whole prior row (snapshot) vs the delta (patch),
plus the cost/benefit tradeoffs.

Each in-tree impl tagged with the choice it made and *why*, so a
future maintainer doesn't have to re-litigate it:

- [`rubix-tools/src/user/store.rs`](../../../crates/rubix-tools/src/user/store.rs) — **snapshot** (small row, security-relevant, infrequent)
- [`rubix-tools/src/team/store.rs`](../../../crates/rubix-tools/src/team/store.rs) — **patch** (members[] is a set; whole-row diffs would explode)
- [`rubix-tools/src/dashboard/store.rs`](../../../crates/rubix-tools/src/dashboard/store.rs) — **snapshot** (revisions are already the unit of change)
- [`rubix-tools/src/flow_ops/store.rs`](../../../crates/rubix-tools/src/flow_ops/store.rs) — **snapshot** today, candidate for patch under Phase B `flow_nodes` storage

### §3.1 — Dashboard rename undo (was silently broken)

The proposal flagged a "verify scope" task for `DashboardReversible`.
Reading the code surfaced a real bug: `change_for` in both
[`rubix-tools/src/dashboard/update.rs`](../../../crates/rubix-tools/src/dashboard/update.rs)
and
[`rubix-tools/src/dashboard/patch.rs`](../../../crates/rubix-tools/src/dashboard/patch.rs)
populated `before.title` / `before.tags` from the **request**, i.e.
the *new* metadata the caller was writing. An undo of a rename
("I just renamed the wrong dashboard") would have restored the
prior body but kept the new title — a silent metadata clobber.

Fix shape:

1. `UpdateDashboardResponse` and `PatchDashboardResponse` now carry
   `prior_title: Option<String>` and `prior_tags: Option<Vec<String>>`
   alongside the existing `prior_body_json` — populated atomically
   by the chokepoint `insert_revision_with_prior` so no follow-up
   round-trip is needed.
2. `change_for` reads `prior_title` / `prior_tags` from the response
   instead of guessing from the request. Patch additionally uses the
   prior values on **both** sides of the snapshot (patch never
   mutates metadata, so `after.title == before.title`).
3. New regression test
   [`dashboard::update::tests::change_for_before_carries_prior_title_and_tags_not_new_ones`](../../../crates/rubix-tools/src/dashboard/update.rs)
   pins the contract.

Patch's existing comment claiming "metadata fields are empty by
design — the inverse path leaves the row's stored title / tags
alone" was a half-truth: the snapshot *was* empty, but
`DashboardSnapshot::into_new_revision` (the inverse path) reads
straight from the snapshot, so empty title/tags would have landed in
the restored row. The fix carries the prior metadata so the inverse
path applies them correctly.

### §3.4 — Redo across processes

In-memory cursor → durable per-actor cursor with epoch CAS:

- Migration:
  [`crates/starter-undo/migrations/postgres/0001_init.sql`](../../../../crates/starter-undo/migrations/postgres/0001_init.sql)
  — `starter_undo_cursors(actor_key, redo_stack JSONB, epoch, updated_at)`.
- Impl:
  [`crates/starter-undo/src/cursor_postgres.rs`](../../../../crates/starter-undo/src/cursor_postgres.rs)
  — `PgUndoCursor` with `UPDATE … WHERE epoch = $observed` race
  protection, retries up to `MAX_CAS_RETRIES = 3`, surfaces
  `Error::Conflict` after exhaustion (no silent spin).
- Task-local actor:
  [`crates/starter-undo/src/actor_local.rs`](../../../../crates/starter-undo/src/actor_local.rs)
  — `tokio::task_local!` so the REST handler can install the caller
  without threading `Actor` through every `Tool::invoke` signature.
- New verb:
  [`rubix-tools/src/undo/redo.rs`](../../../crates/rubix-tools/src/undo/redo.rs)
  — `rubix.undo.redo` mirrors `rubix.undo.last`, calling the new
  `starter_undo::redo_last` helper.
- Integration tests (`#[ignore = "requires docker"]`):
  [`crates/starter-undo/tests/cursor_postgres.rs`](../../../../crates/starter-undo/tests/cursor_postgres.rs)
  — 5 tests including a `concurrent_push_both_land` that exercises
  the CAS path with two writers.

### §3.5 — Per-kind retention policy

[`rubix/crates/rubix-store-postgres/migrations/undo/0003_undo_kind_policy.sql`](../../../crates/rubix-store-postgres/migrations/undo/0003_undo_kind_policy.sql)
— new `undo_kind_policy(resource_kind PK, max_rows_per_resource, max_age_days)`
table, seeded with the curves the proposal calls out:

- `user`, `team` → 50 rows / 180 days (security-relevant, "I demoted the wrong person last week")
- `flow_def` → 200 rows / 30 days (chatty during authoring; old revisions reconstructable from changelog)

[`rubix-agent/src/boot/undo_sweep.rs`](../../../crates/rubix-agent/src/boot/undo_sweep.rs)
rewritten to `LEFT JOIN undo_kind_policy ON p.resource_kind = r.resource_kind`
+ `COALESCE`. Adding a new kind never requires seeding a policy row
— it just inherits the boot-config defaults. No Rust branches.

### §3.6 — Drop `tenant` Reversible

[`rubix/crates/rubix-store-postgres/migrations/undo/0002_drop_tenant_kind.sql`](../../../crates/rubix-store-postgres/migrations/undo/0002_drop_tenant_kind.sql)
— drops `'tenant'` from the `undo_snapshots.resource_kind` CHECK
constraint. Rationale already in proposal §3.6: tenant
delete/restore is rare, operator-driven, and the recovery path is
backup + audit-log replay, not `rubix.undo.last`.

### Prod boot wiring (the proposal's "outstanding gap")

The proposal v2 had a banner at the top of `docs/design/undo/README.md`
saying *"the production runtime does not wire any of this up"*. Now
wired end-to-end:

1. [`rubix-agent/src/registry.rs`](../../../crates/rubix-agent/src/registry.rs)
   — new `UndoSubstrate { recorder, log, cursor }` struct.
   `build_tool_registry` takes it as a 6th arg, builds a
   `ReversibleRegistry` covering all 11 reversible verbs, wraps each
   via `UndoDispatcher`, and appends `rubix.undo.last` +
   `rubix.undo.redo`. The wrap helper uses
   `Arc<dyn ReversibleTool>` — `UndoDispatcher<T>` got `T: ?Sized`
   to make this work.
2. [`rubix-agent/src/routes/tools.rs`](../../../crates/rubix-agent/src/routes/tools.rs)
   — REST handler installs the caller's `Actor` into the
   `CURRENT_ACTOR` task-local for the duration of the
   `Tool::invoke`. `Actor::User { subject }` for authenticated
   callers, falls back to `Actor::System` when no principal.
3. [`rubix-agent/src/main.rs`](../../../crates/rubix-agent/src/main.rs)
   — constructs the substrate from the live PG pool when present
   (`PgChangeRecorder` + `PgChangeLog` + `PgUndoCursor`), runs the
   `starter_undo_cursors` migration alongside changelog migrations,
   threads into `build_tool_registry`. `Option<…>` so the
   no-Postgres laptop path skips wiring and falls back to
   changelog-only behaviour.
4. [`rubix-agent/Cargo.toml`](../../../crates/rubix-agent/Cargo.toml)
   — `starter-undo = { workspace = true, features = ["postgres"] }`
   promoted from dev-dep to main dep.
5. [`rubix-agent/src/boot/mcp/register.rs`](../../../crates/rubix-agent/src/boot/mcp/register.rs)
   — laptop MCP-fallback path now passes the 6th `None` arg (no undo
   wiring under the stdio `rubix-admin mcp` subcommand).
6. [`rubix/docs/design/undo/README.md`](../../design/undo/README.md)
   — replaced the "boot wiring missing" banner with the
   now-accurate wiring description.

## Validation

- `cargo build --workspace` — clean.
- `cargo test -p rubix-tools --lib dashboard` — 54 / 54 pass, including the new rename regression test.
- `cargo test -p rubix-agent --test undo_dispatch_test` — 3 / 3 pass (added two: redo-clear-on-mutation invariant + unregistered-kind cursor-untouched guard).
- `cargo check -p starter-undo --features postgres` — clean.
- `cargo test -p starter-undo --features postgres --test cursor_postgres -- --ignored --test-threads=1` — 5 / 5 pass against ephemeral docker Postgres (`testcontainers`).

## Proposal §3.4 redo-clear-on-mutation contract (this session)

Gap found while writing the e2e plan: `record_if_reversible` does
not touch the cursor, so a new mutation by an actor would NOT have
cleared their redo stack. Proposal §3.4 mandates this clear. Fix
shape:

- `UndoDispatcher` grew an optional cursor field (`Option<Arc<dyn UndoCursor>>`).
- New constructor `UndoDispatcher::with_cursor(...)` for the
  production wiring path; the existing `UndoDispatcher::new(...)`
  stays cursor-less (backwards-compatible with the 9 in-tree
  callsites in unit / integration tests of individual reversible
  tools).
- `invoke_with_group` calls `cursor.clear_redo(actor)` ONLY after
  `record_if_reversible` returns `Ok(Some(_))` — read-only or
  unregistered verbs do not invalidate the cursor.
- `rubix-agent::registry::build_tool_registry` now uses
  `with_cursor` and threads the live `PgUndoCursor` from the
  `UndoSubstrate`.
- Two new tests in `rubix-agent/tests/undo_dispatch_test.rs` pin
  the contract: one proves the stack is cleared after a successful
  mutation, one proves an unregistered kind leaves the stack alone.

## What's NOT done (next-session candidates)

In priority order — see also the "Concrete next steps" list at
[`rubix/docs/proposal/flow-storage-and-undo-redo.md`](../../proposal/flow-storage-and-undo-redo.md)
§3 lines 274-283:

### 1. Live-test the end-to-end undo / redo flow against the agent

**DONE in follow-up session 2026-05-29.** Landed as
[`rubix/crates/rubix-agent/tests/undo_redo_e2e_test.rs`](../../../crates/rubix-agent/tests/undo_redo_e2e_test.rs).
A single `#[ignore = "requires docker"]` integration test pins
the full sequence against testcontainers Postgres:

1. Create + update a dashboard through `UndoDispatcher::with_cursor`.
2. `undo.last` → title reverts; redo stack has the v2 group.
3. `undo.redo` → title re-applies; redo stack empty.
4. Undo again, then issue a *new* update; assert the redo stack is
   cleared and the next `undo.redo` returns `Error::NotFound`
   (the §3.4 clear-on-mutation contract, end-to-end).
5. Undo once more, drop the service + cursor, rebuild a fresh
   `PgUndoCursor` and `UndoService` from the same pool, and call
   `redo` again — proves the redo stack survives a "process
   restart" at the SQL layer.

Verified locally:
`cargo test -p rubix-agent --test undo_redo_e2e_test -- --ignored`
→ `1 passed; 0 failed`.

Two long-term decisions landed alongside the test:

- **Cursor migration consolidated into the canonical boot chain.**
  Removed the inline `migrate(pool).with_source(undo_cursor_migration_source())`
  from `main.rs` and added it to the `sources = [...]` array in
  [`boot/migrations.rs`](../../../crates/rubix-agent/src/boot/migrations.rs).
  All schema now flows through one path; an operator running
  `apply_migrations` provisions `starter_undo_cursors` too.
- **Testcontainers Postgres pinned to `17-alpine`.** The
  testcontainers-modules 0.11 default is `postgres:11-alpine`,
  which lacks core `gen_random_uuid()` and broke every
  dashboards-touching integration test. Pinning a modern line in
  [`starter-store-postgres/src/testing/with_database.rs`](../../../../crates/starter-store-postgres/src/testing/with_database.rs)
  removes the need for per-migration `CREATE EXTENSION pgcrypto`
  hacks and aligns with the deployment matrix.

### 2. Dashboard `definition` parent-row Reversible — DECISION RECORDED

The metadata-fold tradeoff is now documented at
[`rubix/docs/design/undo/README.md`](../../design/undo/README.md)
under "Dashboard metadata fold". Action item: revisit when a
metadata-only verb appears, or when a rename-history view wants
metadata edits in isolation.

### 3. §3.3 — User role / prefs Reversible

The proposal explicitly says this needs an audit-log proposal
**first**, not bolted onto undo. Don't extend undo retention to
substitute for audit. Two separate systems.

Concrete deferral: ship the audit-log proposal under
`rubix/docs/proposal/` before touching `UserReversible`. Without
the audit log, role changes have undo (which expires) but no
permanent record (which doesn't) — wrong-shaped security posture.

### 4. §3.2 — Node-level undo inside a flow

Deferred until Phase B `flow_nodes` / `flow_edges` relational
storage lands (see proposal §2). Today `FlowDefReversible`
snapshots the whole YAML on deploy; per-node patch granularity
becomes natural once `INSERT/UPDATE/DELETE flow_nodes` is the unit
of change.

### 5. Operator surface for `undo_kind_policy`

The table exists, seeded curves exist, sweep reads it — but no
admin verb (`rubix.undo.policy.set` or similar) exists yet, so
operators tune via raw SQL. Low priority; ship when an operator
asks.

### 6. CI wiring for the docker-backed cursor tests

`cargo test -p starter-undo --features postgres --test cursor_postgres -- --ignored`
passes locally with the docker daemon up, but CI isn't yet wired
to run the `--ignored` set against a service container. Mirror
the pattern other crates use in
[`rubix/Makefile`](../../../Makefile) `make test-integration` and
add the corresponding GH Actions / Forgejo step.

## Files touched (full list)

### Schema / migrations

- `crates/starter-undo/migrations/postgres/0001_init.sql` — NEW
- `rubix/crates/rubix-store-postgres/migrations/undo/0002_drop_tenant_kind.sql` — NEW
- `rubix/crates/rubix-store-postgres/migrations/undo/0003_undo_kind_policy.sql` — NEW

### Library code

- `crates/starter-spi/src/changelog/reversible.rs` — policy doc-comment
- `crates/starter-undo/src/lib.rs` — `pub mod actor_local`, `cursor_postgres`, `redo_last` helper
- `crates/starter-undo/src/cursor_postgres.rs` — NEW
- `crates/starter-undo/src/actor_local.rs` — NEW
- `crates/starter-undo/Cargo.toml` — split sqlx feature flags; postgres test gate
- `crates/starter-undo/tests/cursor_postgres.rs` — NEW (5 #[ignore] tests)
- `rubix/crates/rubix-spi/src/dto/dashboard/update.rs` — `prior_title` / `prior_tags`
- `rubix/crates/rubix-spi/src/dto/dashboard/patch.rs` — `prior_title` / `prior_tags`
- `rubix/crates/rubix-tools/src/dashboard/update.rs` — populate + use new fields + regression test
- `rubix/crates/rubix-tools/src/dashboard/patch.rs` — populate + use new fields
- `rubix/crates/rubix-tools/src/user/store.rs` — policy tag
- `rubix/crates/rubix-tools/src/team/store.rs` — policy tag
- `rubix/crates/rubix-tools/src/dashboard/store.rs` — policy tag
- `rubix/crates/rubix-tools/src/flow_ops/store.rs` — policy tag
- `rubix/crates/rubix-tools/src/undo/redo.rs` — NEW
- `rubix/crates/rubix-tools/src/undo/mod.rs` — register `redo`
- `rubix/crates/rubix-tools/src/undo/dispatch.rs` — `T: ?Sized` + `LocalActor` + `with_cursor` constructor that clears redo on successful mutation

### Wiring

- `rubix/crates/rubix-agent/Cargo.toml` — `starter-undo` to main dep with `postgres` feature
- `rubix/crates/rubix-agent/src/main.rs` — `UndoSubstrate` construction + migration
- `rubix/crates/rubix-agent/src/registry.rs` — `UndoSubstrate` + `wrap_rev` + `rubix.undo.{last,redo}` (now uses `UndoDispatcher::with_cursor`)
- `rubix/crates/rubix-agent/src/routes/tools.rs` — task-local actor installation
- `rubix/crates/rubix-agent/src/boot/mcp/register.rs` — 6th `None` arg
- `rubix/crates/rubix-agent/src/boot/undo_sweep.rs` — `undo_kind_policy` JOIN
- `rubix/crates/rubix-agent/tests/{alert_path_threshold,changelog_middleware,rest_disk}_test.rs` — 6th `None` arg
- `rubix/crates/rubix-agent/tests/undo_dispatch_test.rs` — two new tests pinning the §3.4 redo-clear-on-mutation contract

### Docs

- `rubix/docs/design/undo/README.md` — wiring caveat removed, replaced with current shape
- `rubix/docs/sessions/undo/2026-05-28-undo-redo-landed.md` — this doc
