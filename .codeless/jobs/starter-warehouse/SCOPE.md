# Scope — starter-warehouse

The authoritative design lives in three documents on master:

- [/home/user/code/rust/starter/DOCS/storage/ADR-003-clickhouse-warehouse.md](/home/user/code/rust/starter/DOCS/storage/ADR-003-clickhouse-warehouse.md)
  — the decision to split history (ClickHouse) from dimensions
  (Postgres).
- [/home/user/code/rust/starter/DOCS/Tags/SCOPE.md](/home/user/code/rust/starter/DOCS/Tags/SCOPE.md)
  — the shared tag language (T1–T8, D1–D7).
- [/home/user/code/rust/starter/DOCS/Warehouse/SCOPE.md](/home/user/code/rust/starter/DOCS/Warehouse/SCOPE.md)
  — the capability spec (W1–W16, RF/M/BI cross-references from
  the two peer reviews).

This brief is the per-job scope. **Where this disagrees with the
specs, the specs win** — fix this file rather than diverge. The
specs have been through two independent peer reviews; the design
is settled.

## Goal

Land a working `starter-warehouse` capability on the
`codeless/starter-warehouse` branch of the `starter` repo. After
this job:

1. `starter-tags` crate exists with `TagSet`, `TagQuery`, and
   three compile targets (PG, CH, in-process) all agreeing on
   truth value under the D6 semantic-parity invariant.
2. `starter-store-postgres` gains a `dimensions` feature with
   eight catalog tables behind `_sqlx_migrations_dimensions`.
3. `starter-store-clickhouse` crate exists with the L1/L2 tables,
   the `entities_dict` dictionary, and the W11 dimension-freshness
   surface.
4. `starter-warehouse` crate exists with every node kind, every
   REST endpoint, every SSE stream, and every load-bearing rejection
   path from the W-rules implemented and tested.
5. `examples/flow-agent` opts into the warehouse capability and
   smokes end-to-end.
6. `examples/iot-anomaly-detector` is ported to the worked-example
   shape: MQTT flow → cleaner → mart → mart.read-driven verdicts.
   No raw ClickHouse SQL in the binary.
7. The spec's W-rules are not aspirational prose — each is covered
   by a passing test. The final handover names the W-rule ↔ test
   matrix.

## In scope (five slices)

### Slice A (stage 1) — `starter-tags`

- New `crates/starter-tags/` (sync, no DB drivers, no tokio).
  Deps: `serde`, `serde_json`, `thiserror`, `nom`.
- `TagSet` / `TagValue { Bool | Str }` — **no `Num` variant** per
  T2. `tag_value_to_ch_string` is the single canonical converter.
  `TagSet::insert` rejects `Str("true"/"false")` and case
  variants per the Bool/Str reserved-string rule (closes M-2 from
  the second peer review). NaN/Inf and non-integer JSON numbers
  rejected at construction.
- `TagQuery` AST + nom parser per T7. Integer literals only;
  float literals rejected at parse time with a typed error
  pointing the writer at `samples.value_num`.
- `compile_pg`, `compile_ch`, `compile_match` — pure functions,
  zero DB drivers linked.
- `TagDefinition` with `kind` enum
  `'bool' | 'str' | 'ref' | 'num_discriminant'` (per T5
  reconciliation — no bare `'num'`).
- `reserved.rs` for the T6 reserved-key table.
- Test suite including `tests/semantic_parity.rs` — the D6
  invariant. Fixtures: integer-as-string discriminant, Bool,
  bare-tag sugar, float-literal rejection, Bool/Str reserved
  rejection.
- File-size budget per Tags SCOPE.

### Slice B (stage 2) — `starter-store-postgres["dimensions"]`

- New `dimensions` feature on the existing crate's `Cargo.toml`.
- Eight migrations under `migrations/dimensions/`:
  - `0001_entities.sql` — JSONB tags + GIN `jsonb_path_ops`.
  - `0002_entity_refs.sql` — PK `(from_id, rel, to_id)` + FK
    CASCADE both directions.
  - `0003_tag_definitions.sql` — T5 schema with 4-value kind
    CHECK.
  - `0004_tag_prefix_registry.sql` — T6 BI-4 with `prefix`
    PRIMARY KEY.
  - `0005_marts.sql` — W5 schema including `promoted_columns
    TEXT[]`, status CHECK, created_by CHECK, the
    partial-index-backed live-quota trigger from W12.
  - `0006_cleaners.sql` — backfill enum, validate_entity enum,
    `mv_live_at`, `backfill_window_end`, source-freeze mirror
    fields.
  - `0007_sandboxes.sql` — `columns_revision`,
    `frozen_at_revision`, status CHECK per RF-4.
  - `0008_ext_manifest_approvals.sql` — composite PK
    `(ext_id, manifest_hash)` per W12 RF-5.
- Version table: `_sqlx_migrations_dimensions` (non-default sqlx
  convention — W1 names it).
- `src/dimensions/` module behind `#[cfg(feature = "dimensions")]`,
  one submodule per table plus `catalog_gc` (W15) and
  `catalog_audit` (W5 drift check).
- `DIMENSIONS_MIGRATION_SOURCE` exported.
- Integration tests including: prefix conflict fails the txn
  (BI-4), live-mart quota trigger only scans live rows (W12
  partial-index optimisation).

### Slice C (stage 3) — `starter-store-clickhouse`

- New crate. Deps: official `clickhouse` Rust crate (pinned per
  ADR-003 — **not** `klickhouse`), `tokio`, `reqwest`, `serde`,
  `chrono`, `uuid`, `starter-tags`, `testcontainers` under a
  `testing` feature.
- Five migrations under `migrations/`:
  - `0001_raw_events.sql` — `id UInt64 DEFAULT generateSnowflakeID()`
    per M-1, ZSTD codec for parts >3 days, `PARTITION BY
    toYYYYMMDD(received_at)`, `TTL 14 DAY`.
  - `0002_samples.sql` — `PARTITION BY toYYYYMM(ts)`, `ORDER BY
    (entity_id, ts)`, `TTL TO VOLUME 's3_cold' 90 DAY` +
    `INTERVAL 2 YEAR DELETE`.
  - `0003_events.sql` — `ORDER BY (kind, entity_id, ts)`, 1 YEAR
    TTL, LowCardinality cardinality-cap comment per M-5.
  - `0004_documents.sql` — caller-supplied `id String`.
  - `0005_entities_dict.sql` — `CREATE DICTIONARY entities_dict
    SOURCE(POSTGRESQL(…)) LIFETIME(MIN 300 MAX 600)
    invalidate_query SELECT max(updated_at) FROM entities
    LAYOUT(HASHED())`.
- Bloom-filter skip index on `tags` on every history table.
- Small in-crate migration runner (no `sqlx::migrate` equivalent
  for CH). Each file: one DDL, `IF NOT EXISTS` everywhere (CH DDL
  is non-transactional).
- `src/store/{raw_events,samples,events,documents}.rs` — typed
  write paths per W8 (`async_insert=1, wait_for_async_insert=1`
  on every connection). Raw `INSERT` strings outside the store
  crate are forbidden (lint-enforced).
- `src/dim_freshness.rs` — the W11 status enum
  `fresh|stale_within_bound|stale_beyond_bound|failed_refresh`
  querying `system.dictionaries` with a 5 s server-side cache.
- `src/testing/with_clickhouse` — testcontainer helper mirroring
  `starter-store-postgres::testing::with_database`.
- Integration tests covering: roundtrip per table, dim_freshness
  status transitions, **W16 read-after-write bound observable in
  ≤ 1.5 s**, W13 `dictGetOrNull` surfaces orphan as NULL.

### Slice D (stage 4) — `starter-warehouse`

- New crate behind a `warehouse` cargo feature default-off.
- **Every node kind from W9** implemented as `NodeBehavior`:
  - `tap.write`, `curate.write` (per-row PG lookup, W7).
  - `bulk.import` — `target` enum
    `'samples' | 'sandbox:<name>' | 'raw_events'` required, no
    default. `async_insert=0` + 10k batch per W8a.
  - `sandbox.define`, `sandbox.redefine` (refused if
    `frozen_at_revision IS NOT NULL`, requires `confirm: true`),
    `sandbox.drop`.
  - `cleaner.define` — backfill enum + `validate_entity` enum;
    sync backfill auto-promotes to async beyond
    `cleaner.sync_backfill_max_rows` (default 1M) with 5-min
    wall-clock cap per RF-6; freezes source sandbox; idempotency
    via `ReplacingMergeTree(version)` or deterministic-key
    requirement per the rewritten W9 rule.
  - `cleaner.promote`, `cleaner.drop` (clears
    `frozen_at_revision` on source sandbox).
  - `mart.define` — writes `promoted_columns` after DDL succeeds
    per RF-1; `ORDER BY (<first group_by>, bucket, <rest>)` per
    W5/D7; `AggregatingMergeTree` target. Manifest-hash check
    per W12 RF-5: a hash change re-quarantines *all* of that
    extension's live marts and cleaners in the same txn.
  - `mart.promote`, `mart.read`, `mart.drop`.
- `mart.read` enforces W14 by reading
  `marts.promoted_columns` (not `group_by`). HTTP 400 with
  structured body naming `promoted_columns` on unsupported keys.
  `range: { from, to }` with `max_buckets` cap (default 20_000)
  per M-4. `hide_unknown=false` default per W13. Envelope
  carries the W11 `dimension_freshness` block with the four-state
  status enum.
- REST surface per the "REST and SSE surface" table exactly.
  `/api/warehouse/status` returns HTTP 503 when any dictionary's
  status is `failed_refresh`. `/api/warehouse/gc` (W15 manual
  trigger). `/api/warehouse/audit` (W5 drift check).
- SSE handlers on `/api/marts/events` and `/api/entities/events`
  via `starter-server::sse::keepalive(15s)`.
- W15 catalog GC daily background task: 90 days for
  quarantined/failed, 365 days for promoted sandboxes.
- MCP tool surface per "AI agent / MCP" — `query_entities`,
  `tag_entity`, `define_mart`, `drop_mart`, `read_mart`,
  `define_sandbox`, `peek_sandbox` — behind an `mcp` sub-feature.
- File-size budget per the "File-size budget" table.
- Integration tests for every node kind happy path **plus** every
  load-bearing rejection path:
  - W14 400 with structured body.
  - W12 manifest-hash re-quarantine of *existing* live ext rows.
  - RF-4 sandbox.redefine refusal when frozen.
  - RF-6 sync→async auto-promotion.
  - W11 status transitions including the 503 path.
  - W16 read-after-write observable via
    `ingest.async_insert_oldest_age_ms`.

### Slice E (stage 5) — flow-agent smoke + iot-anomaly-detector port + sweep

- Wire the warehouse feature into `examples/flow-agent` (under
  a flag — `flow-agent` is Postgres-only for OLTP per ADR-001
  but can opt into the warehouse capability for tagged history).
- Manual smoke: ingest via `tap.write`, define a mart, read it,
  observe `dimension_freshness` in the envelope, capture
  transcript.
- Port `examples/iot-anomaly-detector` to the worked-example
  shape:
  - MQTT subscribe → `tap.write` per ingest.
  - Inline `starter-ext-iot` extension with a `cleaner.define`
    (`raw_events → samples`) and `mart.define` for `mart_iot_1m`
    + `mart_iot_1h`.
  - Anomaly detector becomes two `mart.read` calls + a
    `compute.zscore` flow node emitting Verdicts.
  - **No direct ClickHouse SQL anywhere in the binary.**
- `examples/iot-anomaly-detector/README.md` updated to reflect
  the architecture: manifest-hash trust gate, dim-freshness
  badge, W14 filter contract.
- Final sweep: `cargo test --workspace`, `cargo clippy
  --workspace --all-features -- -D warnings`, `cargo fmt
  --check` all green.
- Final handover: W-rule ↔ test matrix, deferred items
  (ReplicatedMergeTree multi-replica testing, klickhouse
  evaluation, future entity_refs_dict per M-3), honest note
  on which spec claims are mechanically enforced vs prose-only.

## Out of scope

- **Removing or modifying any existing crate's public API.**
  This job is additive on `starter-store-postgres`, and net-new
  for `starter-tags`, `starter-store-clickhouse`,
  `starter-warehouse`. `starter-store-sqlite` is not touched.
- **`ReplicatedMergeTree` + `clickhouse-keeper` multi-replica
  deployment.** Per W10, available but not on the day-one path.
  Note as a deferred item.
- **ClickHouse Cloud features** (SharedMergeTree, ClickPipes).
  W10 forbids; ADR-003 forbids. Self-hosted OSS only.
- **Streaming ingest (Kafka, Pulsar, CDC).** Per ADR-003 "What
  this ADR does NOT do." Flow engine writes via HTTP.
- **Cross-store PITR.** Per ADR-003 explicitly. Orphan audit is
  the recovery tool.
- **`entity_refs_dict`** per M-3. Day-one path is
  Postgres-first + CH `entity_id IN (…)`. Future work.
- **Klickhouse evaluation.** Per the ADR-003 nit, the official
  `clickhouse` Rust crate is pinned. Do not introduce klickhouse
  as an alternative.
- **A SPI-level conformance test suite that runs the same
  trait-level tests against PG + CH.** Library-level work; its
  own job.
- **Removing `TagValue::Num` from anywhere** — it never existed
  in code; the spec was authored without it. This is a no-op
  carve-out, called out so it's clear no migration is needed.

## Constraints

- **W1 — two stores, one capability.** No third storage crate.
  Tests use existing testcontainer helpers (PG) and the new
  CH testcontainer helper.
- **W4 — `TagQuery` is the only filter language at the read seam.**
  `mart.read`, `GET /api/entities`, `GET /api/history`, and
  authz rules accept `TagQuery` and nothing else. Raw SQL is
  reserved for Insights' `rule.sql`.
- **W7 — ingestion never refuses.** `raw_events` accepts any
  payload. Unknown tags pass through. Malformed values get a
  `quality` flag and a log line.
- **W8 — `async_insert=1` on every CH write connection.**
  Enforced by the store crate. Raw `INSERT` strings outside the
  store crate are forbidden (lint).
- **W10 — Apache-2.0 ClickHouse OSS features only.** No
  Cloud-only functions. `clickhouse` Rust crate is pinned (not
  `klickhouse`) per ADR-003.
- **W14 — `mart.read` filters must reference promoted columns
  only.** Filter validation reads `marts.promoted_columns`, not
  `group_by`. HTTP 400 with structured body naming
  `promoted_columns` on unsupported keys. No transparent reroute
  to `samples`.
- **W16 — read-after-write boundary ≤ 1.5 s.** Observable via
  `/api/warehouse/status.ingest.async_insert_oldest_age_ms`.
  Tests poll, not sleep.
- **T2 — tag values are `Bool | Str` only.** No `Num`. No floats
  in tag queries. `Str("true"/"false")` rejected at construction.
  NaN/Inf rejected.
- **T8 — D6 semantic parity across all three targets** is a
  hard invariant. Diverging optimisation that changes any
  target's truth value is a bug.
- **R1 (file-size budget):** ≤ 400 lines per file. The Tags
  SCOPE and Warehouse SCOPE each carry a per-file target table;
  honour them.
- **R-trio applies** (CLAUDE.md): every stage ends with `checks`,
  `docs`, `git` per the closing trio block in `WORKFLOW.md`.
- **No `--no-verify` or `--force`.** If a pre-commit hook
  fails, fix the cause.
- **MSRV / lint gates green at every stage boundary**:
  `cargo test --workspace`, `cargo clippy --workspace
  --all-features -- -D warnings`, `cargo fmt --check`.

## Deliverables (what "done" looks like)

1. `codeless/starter-warehouse` branch with one commit per stage
   (five stages + two REVIEW handovers = seven commits), pushed.
2. **Slice A acceptance:** `cargo test -p starter-tags` green
   including the D6 semantic-parity fixture. No DB driver
   reachable in `cargo tree -p starter-tags`.
3. **Slice B acceptance:** `cargo test -p starter-store-postgres
   --features 'dimensions testing' -- --ignored` green. The
   eight migrations apply cleanly. Prefix-conflict test and
   live-mart quota trigger test both pass.
4. **Slice C acceptance:** `cargo test -p starter-store-clickhouse
   --features testing -- --ignored` green. W16 read-after-write
   bound observed in ≤ 1.5 s in a real test (not a TODO comment).
   W13 `dictGetOrNull` surfaces orphan as NULL.
5. **Slice D acceptance:** `cargo test -p starter-warehouse
   --features 'warehouse testing' -- --ignored` green. Every
   load-bearing rejection path covered by a passing test
   transcript. The W-rule ↔ test matrix is non-empty by stage
   5 and complete by the final handover.
6. **Slice E acceptance:** `cargo run -p iot-anomaly-detector`
   against the docker-compose stack boots, ingests, and emits
   verdicts. `grep -rn 'SELECT\|FROM\|WHERE' examples/iot-anomaly-detector/src/`
   returns zero matches outside comments and string literals
   that document the migration (i.e. no raw SQL in business
   logic). Final handover ships the W-rule ↔ test matrix.
7. `cargo test --workspace` + `cargo clippy --workspace
   --all-features -- -D warnings` + `cargo fmt --check` green
   at every stage boundary.

## Open questions — RESOLVED (2026-05-23, before start)

### Q1 — One job or split per crate?

**Answer: one job, five stages, two REVIEW gates.**

The four new crates are tightly coupled (`starter-warehouse`
depends on `starter-store-clickhouse` and
`starter-store-postgres["dimensions"]`, which both depend on
`starter-tags`). Splitting per crate would land partial,
non-useful intermediate states on master. The two REVIEW gates
catch the load-bearing failure modes:

- After A+B: tag-layer semantic-parity drift and PG schema drift
  caught before any CH work or warehouse code is written
  against them.
- After D: every W-rule's runtime enforcement verified against a
  real CH testcontainer before `examples/iot-anomaly-detector`
  binds to the surface.

Cap: **60000¢ / 8h** — doubled from the typical 30000¢ / 4h
because the work spans four new crates plus an example port.
Slice A is small (~10%), slice B small (~15%), slice C
medium (~25%), slice D the bulk (~35%), slice E the example
port + sweep (~15%).

### Q2 — Implementation order of node kinds within slice D?

**Answer: write paths first, then read, then lifecycle.**

Order within slice D:
1. `tap.write` + `curate.write` + `bulk.import` (ingest is
   sacred — W7 must hold before anything else lands).
2. `sandbox.*` + `cleaner.*` (curation; depends on `bulk.import`
   for analyst-CSV flows).
3. `mart.define` + `mart.read` + `mart.promote` + `mart.drop`
   (read seam; depends on `cleaner.define` for the standard
   workflow but does not require it for unit tests).
4. REST surface + SSE + GC + audit (glue; depends on all of
   the above).
5. MCP tool surface (last; thin wrapper over the node kinds).

Each step gets its own integration test before the next step
starts. Do not batch nodes into one mega-edit.

### Q3 — Where do extensions live?

**Answer: `starter-ext-iot` is inline inside
`examples/iot-anomaly-detector` for this job.**

The full extension manifest + adapter machinery is its own
design (referenced from the Warehouse SCOPE's "AI agent / MCP"
section as `starter-ext-warehouse`, plus the
`contributes.warehouse` extension shape). That machinery is out
of scope here. For slice E, the iot example registers its
cleaners and marts directly via the warehouse capability API
(simulating what an ext adapter would do) and the
`ext_manifest_approvals` flow is exercised by the slice D test
suite using a fixture extension, not a real one.

A future job — call it `starter-ext-warehouse` — wires the real
extension adapter. The W12 trust seam is fully implemented in
this job; the wiring to real extension manifest hashes is
deferred.

## References

- ADR (authoritative):
  [/home/user/code/rust/starter/DOCS/storage/ADR-003-clickhouse-warehouse.md](/home/user/code/rust/starter/DOCS/storage/ADR-003-clickhouse-warehouse.md)
- Tags SCOPE (authoritative for slice A):
  [/home/user/code/rust/starter/DOCS/Tags/SCOPE.md](/home/user/code/rust/starter/DOCS/Tags/SCOPE.md)
- Warehouse SCOPE (authoritative for slices B–D):
  [/home/user/code/rust/starter/DOCS/Warehouse/SCOPE.md](/home/user/code/rust/starter/DOCS/Warehouse/SCOPE.md)
- Existing PG testcontainer helper (slices B/D tests):
  `/home/user/code/rust/starter/crates/starter-store-postgres/src/testing/with_database.rs`
- Existing PG OLTP migration namespace (must not be touched):
  `/home/user/code/rust/starter/crates/starter-store-postgres/migrations/`
- Existing example to wire (slice E flow-agent smoke):
  `/home/user/code/rust/starter/examples/flow-agent/`
- Existing example to port (slice E iot rewrite):
  `/home/user/code/rust/starter/examples/iot-anomaly-detector/`
- Existing docker compose for CH (slice E):
  `/home/user/code/rust/starter/docker/docker-compose.clickhouse.yml`
- Flow SPI (slice D node impls):
  `/home/user/code/rust/starter/crates/starter-flow-spi/`
- Flow nodes registration (slice D descriptors):
  `/home/user/code/rust/starter/crates/starter-flow-nodes/`
