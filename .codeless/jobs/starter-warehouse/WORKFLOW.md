# Workflow — starter-warehouse

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the three authoritative spec
documents:

- [/home/user/code/rust/starter/DOCS/storage/ADR-003-clickhouse-warehouse.md](/home/user/code/rust/starter/DOCS/storage/ADR-003-clickhouse-warehouse.md)
- [/home/user/code/rust/starter/DOCS/Tags/SCOPE.md](/home/user/code/rust/starter/DOCS/Tags/SCOPE.md)
- [/home/user/code/rust/starter/DOCS/Warehouse/SCOPE.md](/home/user/code/rust/starter/DOCS/Warehouse/SCOPE.md)

## Sequencing

Five stages, two REVIEW gates. Strictly linear except where noted:

- **Slice A** (stage 1) is the tag-language foundation. Pure
  library, no DB drivers. Every later slice depends on it.
- **Slice B** (stage 2) is additive on `starter-store-postgres`.
  Adds a `dimensions` feature with eight catalog tables. Does not
  touch the existing OLTP namespace.
- **REVIEW gate 1** after slices A+B. Catches tag-layer drift
  (which would silently propagate through every later slice) and
  schema drift (which would silently propagate through slice D's
  catalog reads). Cheap defence at the cheapest possible point.
- **Slice C** (stage 3) is the ClickHouse store crate. Brand new
  crate. Depends on slice A for the `TagSet` write paths.
- **Slice D** (stage 4) is the warehouse capability. Depends on
  slices A, B, and C. This is the bulk of the job.
- **REVIEW gate 2** after slice D. Catches every W-rule that was
  supposed to be mechanically enforced but isn't — every claim
  that lives as English in the spec must show up as a passing
  test transcript here.
- **Slice E** (stage 5) is the example wiring + final sweep.

Slices B and C can in principle parallelise (different crates),
but **do not batch them**. Each slice ships as its own commit;
the diff stays reviewable; the handover stays honest.

## Per-stage discipline

Before writing any code in a stage:

1. Re-read the relevant spec section. The spec is the contract;
   this WORKFLOW is the process. If the spec is silent on a
   judgment call, the answer goes in `handover.md` under "spec
   gaps" so the next round of design picks it up.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". The
   biggest risk on this job is silent scope creep — the spec
   explicitly carves out tempting work (klickhouse evaluation,
   `entity_refs_dict`, SPI conformance suite, real extension
   adapter, ReplicatedMergeTree). Stay within the carve-outs.
3. For **stage 1** (slice A): write the test suite outline first.
   The D6 semantic-parity fixture is the load-bearing correctness
   property of the entire tag layer; if you can't articulate the
   fixture before you write code, you don't understand T2/T7/T8
   yet. Re-read.
4. For **stage 2** (slice B): start with the migrations. Apply
   each to an empty Postgres via the testcontainer helper. Each
   migration is small; verify each in isolation before moving on.
   The W12 partial-index trigger and the BI-4 prefix-conflict
   constraint are the two tests that prove the schema is alive.
5. For **stage 3** (slice C): start with `with_clickhouse` — the
   testcontainer helper — because every other test depends on it.
   Then the migrations one at a time. Then the typed store paths,
   one table at a time. Then `dim_freshness`. Then the W16 timing
   test, which is the most important test in this stage.
6. For **stage 4** (slice D): write the node kinds in the order
   from SCOPE Q2 — write paths first (`tap.write`, `curate.write`,
   `bulk.import`), then curation (`sandbox.*`, `cleaner.*`), then
   read (`mart.*`), then the REST/SSE/GC/audit glue, then MCP.
   Each step gets its own integration test before the next step
   starts. **Do not batch nodes into one mega-edit.**
7. For **stage 5** (slice E): start by re-reading the worked
   example walkthrough in chat (or equivalent transcript in
   `handover.md` if preserved). The iot port is a *reshape*, not
   a *rewrite* — the existing binary's logic structure
   (baseline + recent + z-score) maps cleanly onto the new node
   surface. The change is *where* the SQL lives, not what it
   computes.
8. `cargo check --workspace` from the `starter` repo root before
   any edit so the baseline is known-clean.

Before committing a stage:

1. `cargo fmt --all` clean.
2. `cargo clippy --workspace --all-features -- -D warnings`
   green.
3. `cargo test --workspace` green. Postgres- and CH-backed
   `#[ignore]` tests run via the appropriate `--features … --
   --ignored` invocation; they must pass against the
   testcontainer helpers.
4. For stages 1–4: update the **W-rule ↔ test matrix** in
   `handover.md` — for every W-rule (or T-rule, or RF/M/BI
   finding) this stage exercises, name the test that proves it.
   This matrix is the load-bearing artefact of the REVIEW gates
   and the final exit summary.
5. For stage 4: if you implemented a node kind without a
   corresponding load-bearing-rejection test (W14 400, W12
   re-quarantine, RF-4 freeze, RF-6 auto-promote, W11 503, W16
   read-after-write), the stage is not done. Each rejection
   path is a discrete test.
6. For stage 5: `cargo tree -p iot-anomaly-detector | grep -i
   clickhouse` shows clickhouse reachable only via
   `starter-store-clickhouse`. `grep -rnE 'SELECT|FROM|WHERE'
   examples/iot-anomaly-detector/src/` returns zero matches
   outside string literals that document the migration.

Commit + push directly to the branch (this repo doesn't use the
mani wrapper):

```
git add -A && \
  git commit -m "stage N: <one-line title from template.yaml>" && \
  git push -u origin codeless/starter-warehouse
```

No `--force`, no `--no-verify`. If a hook fails, fix the cause.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — `cargo fmt --check` + `cargo clippy --workspace
   --all-features -- -D warnings` + `cargo test --workspace`.
   Stages 2, 3, 4 additionally run the relevant `--ignored`
   tests via `cargo test … --features … -- --ignored`. Stage 5
   additionally runs a manual `cargo run -p iot-anomaly-detector`
   smoke against docker-compose.
2. `docs` — update `handover.md` for the next stage, including
   the W-rule ↔ test matrix entries added this stage. Stages 1
   and 4 write into a "spec gaps" section if anything in the
   spec was ambiguous enough that the implementation made a
   judgment call. Module docstrings in any new code cite the
   relevant W/T rule by number.
3. `git` — stage the changes, commit with `stage N: <title>`,
   push to `codeless/starter-warehouse`. One stage, one commit.

A stage is not "done" until all three are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry —
do not mark the stage `[x]`, do not advance, and never `--force`
or `--no-verify`.

## REVIEW gates (two)

### Gate 1 — after slices A+B (stage 2), before slice C

At the gate write a handover comment containing:

- `cargo test -p starter-tags` transcript, green. Include the
  semantic-parity fixture output explicitly so the gate can see
  every fixture row passed.
- `cargo test -p starter-store-postgres --features 'dimensions
  testing' -- --ignored` transcript, green.
- The "tag/dialect translation notes" section: anything subtle
  in the PG dialect rendering of `compile_to_pg`, anything
  subtle in the CH dialect rendering of `compile_to_ch`.
- `git diff master -- crates/starter-store-sqlite` transcript
  showing **empty** — the SQLite crate must be unchanged.
- `git diff master -- crates/starter-store-postgres/migrations/`
  scoped to the existing OLTP namespace showing **empty** — only
  the new `dimensions/` subdir gets new files.
- The W-rule ↔ test matrix so far: every T-rule (T1–T8) and
  every W-rule that slice B touched (W1, W5, W6 schema, W12
  partial-index trigger, W15 GC, plus T6 BI-4 prefix registry
  enforcement) maps to a passing test.

Gate question: *does the tag layer satisfy D6 semantic parity,
and do the catalog tables enforce every constraint the spec
relies on (live-mart quota, prefix uniqueness, status CHECK,
created_by CHECK)?* If any answer is "not yet", the gate fails
— fix and re-request, do not advance.

Do not start slice C without explicit approval at this gate.

### Gate 2 — after slice D (stage 4), before slice E

At the gate write a handover comment containing:

- `cargo test -p starter-warehouse --features 'warehouse
  testing' -- --ignored` transcript, green.
- An explicit transcript for **each** of the following
  load-bearing rejection paths:
  - **W14 400 with structured body.** Run a `mart.read` against
    a real mart with an unsupported filter key; show the full
    HTTP 400 body naming `promoted_columns`.
  - **W12 manifest-hash re-quarantine.** Install a fixture
    extension at manifest hash A; `mart.define` lands a mart
    `live`; bump the manifest hash to B without re-approval;
    show the previously-live mart now `quarantined` (catalog
    SELECT) and `mart.read` against it now errors.
  - **RF-4 sandbox.redefine refusal when frozen.**
    `cleaner.define` from a sandbox; observe
    `sandboxes.frozen_at_revision` becomes non-NULL; attempt
    `sandbox.redefine`; show the refused response.
  - **RF-6 sync→async auto-promotion.** Run `cleaner.define
    { backfill: 'sync' }` against a source with >
    `sync_backfill_max_rows` rows (lower the config for the
    test); show the structured note `{"backfill_mode":
    "async", "reason": "row_count_exceeded", …}`.
  - **W11 dim_freshness status transitions.** Capture
    `/api/warehouse/status.dimensions.entities_dict` in each
    of the four states: fresh, stale_within_bound,
    stale_beyond_bound, failed_refresh. The last one MUST
    show HTTP 503 from `/api/warehouse/status`.
  - **W16 read-after-write.** A `tap.write` followed by
    polling `/api/warehouse/status` for
    `ingest.async_insert_oldest_age_ms = 0`; then `mart.read`
    sees the row. Total wall-clock from write to visible read
    must be ≤ 1.5 s. Capture the timing.
- The W-rule ↔ test matrix complete for slices A through D.
  Every W-rule (W1–W16) and every T-rule (T1–T8) and every
  load-bearing RF/M/BI finding (RF-1 through RF-6, M-1
  through M-5, BI-1, BI-2, BI-4) names the test that proves it.
- `cargo tree -p starter-warehouse --features warehouse` showing
  the dep tree (no `klickhouse`, no Cloud-only features
  reachable).
- A list of every spec claim that is **not** mechanically
  enforced — claims that live only as prose and rely on
  reviewer/operator discipline. The reviewer at this gate
  decides whether each one is acceptable as prose-only.

Gate question: *is every W-rule from the spec runtime-enforced
or explicitly accepted as prose-only?* If any answer is "I'll
get to it in slice E", the gate fails — fix and re-request,
do not advance.

Do not start slice E (example port) without explicit approval
at this gate. The iot example binds to the surface that this
gate validates; binding to an under-tested surface compounds.

## Anti-patterns specific to this job

- **Do not introduce a `TagValue::Num` variant** "just for now"
  intending to remove it later. The variant was deliberately
  removed in the spec because it is a footgun (M-2 from the
  second peer review). If the implementation tempts you to add
  it, you have a different bug; surface in chat.
- **Do not** add a `klickhouse` dependency. The official
  `clickhouse` Rust crate is pinned per ADR-003. If the official
  crate's API is awkward, write a small wrapper inside
  `starter-store-clickhouse` — do not introduce a second client.
- **Do not** make `mart.read` transparently fall back to a
  `samples` scan when a filter key isn't promoted. W14 forbids
  this; the spec is explicit. Return HTTP 400 with the structured
  body and let the caller decide.
- **Do not** silently fall back to `POPULATE` for cleaner
  backfill. W9 / cleaner.define explicitly forbids it. The
  explicit `INSERT … SELECT WHERE ts < <backfill_window_end>`
  path is the only sanctioned backfill.
- **Do not** weaken the W12 re-quarantine to "only new marts"
  on manifest-hash change. RF-5 closed the asymmetry
  deliberately: a manifest-hash change re-quarantines *all* of
  that extension's live rows, not just the new ones.
- **Do not** widen any spec contract to accommodate an
  implementation difficulty. The W-rules are stable. If a
  rule's enforcement is hard to implement, surface — that's a
  design conversation, not a sneak edit.
- **Do not** use ClickHouse `MATERIALIZED VIEW … POPULATE` for
  any DDL. Use plain `CREATE MATERIALIZED VIEW … TO <target>`
  and the explicit backfill `INSERT … SELECT` for cleaners with
  `backfill != 'none'`.
- **Do not** test W16 read-after-write with a `sleep(2000)` in
  the test. The whole point of W16's
  `ingest.async_insert_oldest_age_ms` exposure is that tests
  poll. Sleeping makes the test flaky on slow CI and useless as
  a correctness signal.
- **Do not** test `dim_freshness` by hardcoding timestamps. The
  status transitions are real ClickHouse state transitions;
  drive them by killing the Postgres connection (for
  `failed_refresh`), by waiting past `lifetime_max` (for
  `stale_beyond_bound`), and by an initial load (for `fresh`).
- **Do not** stuff the iot example back into a polling loop in
  slice E. The point of the port is that the binary is a thin
  flow driver — `mqtt.subscribe` → `tap.write` for ingest, two
  `mart.read` + a `compute.zscore` for detection. If the binary
  has a `loop { sleep … query … }` it's the wrong shape.
- **Do not** wire the real extension adapter in slice E. SCOPE
  Q3 explicitly defers `starter-ext-warehouse` to a future job.
  The iot example registers its cleaners and marts directly via
  the warehouse capability API for this job.
- **Do not** skip the W-rule ↔ test matrix in any handover.
  The matrix is the load-bearing artefact of the gates and the
  final exit summary. A handover without the matrix is an
  incomplete handover.

## When to halt

- **Stage 1** finds that the nom parser cannot cleanly reject
  float literals at parse time without breaking the integer
  literal path. Halt; the T7 grammar is the spec; surface the
  parser-shape problem and propose either a grammar refinement
  or a post-parse validation pass. Do not silently accept floats
  and validate later — the doc is explicit they are a parse
  error.
- **Stage 2** finds that the partial-index-backed live-mart
  quota trigger has a race condition under concurrent
  `mart.define` calls. Halt; document the race shape and propose
  either an advisory lock or a serialisable txn boundary. The
  live-quota guarantee is in the spec; don't relax it.
- **Stage 3** finds that the `generateSnowflakeID()` default is
  unavailable in the testcontainer's ClickHouse version. Halt;
  bump the testcontainer image tag (CH 24.6+) and document the
  minimum required CH version in `starter-store-clickhouse`'s
  README. Do not silently emit client-side IDs as a workaround.
- **REVIEW gate 1** fails because the D6 semantic-parity fixture
  has a row where T8a and T8b disagree. Halt; fix the compiler
  divergence. Semantic parity is non-negotiable; an undetected
  divergence here corrupts every later read.
- **Stage 4** finds that the W16 read-after-write bound is not
  achievable in ≤ 1.5 s on the testcontainer (e.g. the CH
  default async_insert_busy_timeout_ms is higher than the spec
  assumes). Halt; either tune the CH server config in the
  testcontainer setup to match the spec, or revise W16's bound
  in the spec with an explicit doc update and re-review. Do not
  silently widen the test's tolerance.
- **REVIEW gate 2** fails because one of the load-bearing
  rejection paths has no test. Halt; write the test, then
  re-request. Do not advance to the iot port with an
  unenforced W-rule — the iot port binds to the surface that
  gate 2 validates.
- **Stage 5** finds that the iot port needs a node kind that
  doesn't exist in slice D (e.g. a `compute.zscore` shape
  that's awkward as a generic flow node). Halt; surface — either
  the node is generic enough to live in `starter-flow-nodes`
  (file a follow-up) or it's iot-specific and lives in the
  example crate itself.
- **Stage 5** finds that the docker-compose smoke for the iot
  example is flaky against the CI runner. Halt; reduce to a
  unit-test-able shape (in-process MQTT broker via rumqttd, an
  in-process CH testcontainer) and document the smoke as a
  local-only recipe. Do not silently mark the smoke as skipped.
- **Budget** is blown before slice E. Halt at REVIEW gate 2.
  Split slice E off as `starter-warehouse-iot-port`. Do not
  silently land a partial iot port; the half-ported state is
  worse than a clean intermediate where the warehouse is done
  but the iot example is unchanged.
