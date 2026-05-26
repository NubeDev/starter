# Scope — warehouse-engine-swap

The authoritative design lives at
[/home/user/code/rust/starter/rubix/docs/proposal/warehouse-engine-swap.md](/home/user/code/rust/starter/rubix/docs/proposal/warehouse-engine-swap.md).
This brief is the trimmed per-job scope. Where this disagrees with the
proposal, **the proposal wins** — fix this file rather than diverge.

## Goal

Swap the time-series warehouse engine from ClickHouse to TimescaleDB
and rename the surface to be vendor-neutral (`warehouse` instead of
`clickhouse`). After this job:

1. ClickHouse is **gone** from the codebase and compose stack — crate,
   dialect, service, env vars, CI matrix, the `entities_dict`
   dictionary, the `async_insert` discipline, and the `clickhouse`
   verb namespace.
2. The warehouse surface uses neutral names: `rubix.warehouse.*`
   verbs, `WarehouseWriter` trait, `Warehouse{Rule,Mart,Retention}Reversible`
   snapshots, `starter-store-warehouse` crate, `warehouse-ruler` skill,
   `warehouse-ruler.yaml` flow, `rubix_tools::warehouse::mart` module.
3. `MartSpec` DDL generation goes through a `DdlDialect` trait with a
   single `TimescaleDb` impl. Marts emit continuous-aggregate DDL.
4. Retention runs through `add_retention_policy` /
   `remove_retention_policy`; the reversible snapshot reads from
   `timescaledb_information.jobs`.
5. Rule verb snapshots read from
   `timescaledb_information.continuous_aggregates` joined with
   `pg_get_viewdef` (no more `SHOW CREATE TABLE`).
6. The `entities_dict` materialized dictionary and its refresh job
   are deleted. Mart queries JOIN directly against dimension tables
   in the same Postgres instance.

## In scope (three stages mapping to the proposal's three phases)

- **Stage 1 — Rename + decouple.** Pure rename of traits, crates,
  modules, verbs, skill, flow YAML. Old names deleted outright.
  Extract `MartSpec` DDL behind a `DdlDialect` trait. Decide
  `chunk_time_interval` per hypertable. Swap dev compose to
  TimescaleDB. Audit existing `MartSpec` against cagg constraints.
- **Stage 2 — TimescaleDB implementation.** Wire
  `starter-store-warehouse` to sqlx PgPool. Implement
  `WarehouseWriter` via `COPY` into hypertables. Add
  `DdlDialect::TimescaleDb` emitting continuous-aggregate DDL. Port
  retention and rule verbs. Replace `entities_dict` references with
  direct JOINs.
- **Stage 3 — Delete ClickHouse.** Remove the ClickHouse dialect,
  the docker service, env vars, CI matrix, `entities_dict`, the
  `async_insert` discipline, and any remaining references. Update
  README and setup docs.

## Out of scope

- **Backfill of any data.** Pre-production — there is nothing to
  preserve. Drop and recreate.
- **Dual-write / parallel running / reconciliation.** No production
  traffic to dual-write against.
- **Deprecated verb aliases / forwarding shims.** Old names are
  removed in stage 1, not deprecated.
- **Gating cutover on benchmarks.** Benchmarks are informational
  only (sizing + capacity planning) — see the proposal's
  Benchmarks section.
- **Compressed-chunk backfill runbook.** Deferred until the
  warehouse holds data worth correcting (post-launch).
- **Promoting hot tag keys into columns.** Stay on `jsonb` + GIN for
  the swap; revisit if GIN selectivity proves insufficient under
  real load (post-launch).
- **Touching the relational PostgreSQL schema, auth, or undo
  changelog.** The proposal's "what stays the same" list — ADR-003
  tenancy, `rubix.undo.last`, the resolver-layer tenancy filter,
  retention tiers, the CI lint banning raw INSERT/SELECT outside
  typed paths — all unchanged.
- **Adding a second engine behind `DdlDialect`.** Only the
  TimescaleDb impl exists after this job. The trait shape is
  designed to allow others, but none ship here.
- **MCP-only refactor of the AI surface.** Out of scope for this
  job; tracked separately per the user's long-term direction memo.

## Constraints

- **Pre-production — hard delete is fine.** No callers to protect,
  no data to preserve, no operators to migrate.
- **R1** — 400 lines per file. Apply to any new module created
  during the swap; the rename should not introduce file bloat.
- **R4** — layer arrow `contracts → domain → transport`. The
  `DdlDialect` trait lives in the contracts layer; the
  TimescaleDb impl lives in the domain layer; verb wiring stays
  in transport.
- **R10** — add-only-within-a-major. The verb rename is a
  breaking change, which is acceptable pre-production but should
  be reflected in the version bump on the verb namespace.
- **R11** — tests live with the code. Every new write path,
  cagg emission, retention port, and snapshot read gets a
  testcontainer-backed integration test gated `#[ignore]` per
  the existing starter convention.
- **R12** — comments explain why, never what. No `// TODO`
  litter; no `// renamed from ChWriter` graveyard markers.
- **R13** — drive everything through mani. `mani run build`,
  `mani run test`, `mani run lint`, `mani run status` are the
  contributor entry points across the swap.
- **ADR-003** — per-row `tenant_id` tenancy. Every cagg's
  `GROUP BY` must include `tenant_id`; every cagg is created
  with `security_invoker = true` so RLS works against it.
- **Tenancy stays at the resolver layer.** Direct JOINs that
  replace `entities_dict` do not bypass tenancy — the dimension
  tables they join against are already tenancy-scoped at the
  resolver.
- **No `--force`, no `--no-verify`.** If a hook fails, fix the
  cause.

## Deliverables (what "done" looks like)

1. `codeless/warehouse-engine-swap` branch with one commit per
   stage (three stages = three commits), pushed via mani.
2. `cargo build --workspace` green at every stage boundary.
3. `cargo clippy --workspace --all-features -- -D warnings` green
   at every stage boundary.
4. `cargo fmt --check` green at every stage boundary.
5. `mani run build --all` green; `mani run lint` green; `mani
   run test --all` green at every stage boundary (testcontainer
   `#[ignore]` tests run as part of the stage's `verify:` list).
6. Repo grep for `clickhouse` / `ClickHouse` outside
   `rubix/docs/proposal/warehouse-engine-swap.md` returns no
   matches in live code after stage 3.
7. The TimescaleDB testcontainer smoke covers: ingest into all
   four hypertables (`raw_events`, `samples`, `events`,
   `documents`), mart cagg refresh and query, retention drop,
   rule verb snapshot, undo path.
8. `entities_dict` materialized dictionary and its refresh job
   are gone; mart queries that previously referenced it now
   JOIN directly and the tests that exercise those marts pass.

## Open questions — RESOLVED (2026-05-26, before start)

The proposal is the authoritative resolution. Three job-specific
notes follow.

### Q1 — Single job vs split per phase?

**Answer: single job, three stages, no REVIEW gates.**

Pre-production. Each phase of the proposal is mechanical and
tightly coupled to the next (the rename in stage 1 is the seam
stage 2 fills; deleting ClickHouse in stage 3 only makes sense
after stage 2's TimescaleDB path works). Splitting into three
jobs would force three submit/start cycles for no review
benefit — the user is comfortable letting codeless run for
hours through the whole thing.

### Q2 — Do we keep the `DdlDialect` trait after only one impl ships?

**Answer: yes.**

The trait is cheap to keep and the proposal's "long-term wins"
section names a future engine swap (DuckDB / Citus / other) as
a `DdlDialect` impl. The trait is the load-bearing decoupling
between the rename and the engine swap. Keeping it post-swap
costs nothing; removing it would have to be re-added the first
time someone wants to experiment with another engine.

### Q3 — Does the rename touch external API consumers?

**Answer: no — pre-production, no external consumers.**

The verb rename is a breaking change to anyone wiring against
`rubix.clickhouse.*`. Pre-production, there are none, so the
old names are removed outright with no alias / no
deprecation cycle.

## References

- Proposal (authoritative):
  [/home/user/code/rust/starter/rubix/docs/proposal/warehouse-engine-swap.md](/home/user/code/rust/starter/rubix/docs/proposal/warehouse-engine-swap.md).
- Parent rubix SCOPE: `rubix/SCOPE.md` for R0–R13 and ADR-003.
- Existing testing seams:
  `starter-store-postgres::testing` (used post-swap for the
  TimescaleDB testcontainer — same Postgres testcontainer with
  the `timescaledb` + `timescaledb_toolkit` extensions
  preloaded).
- mani docs in the codeless-workspace:
  [`../../../../codeless-workspace/DOCS/MANI.md`](../../../../codeless-workspace/DOCS/MANI.md).
