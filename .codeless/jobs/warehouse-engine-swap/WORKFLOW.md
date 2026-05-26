# Workflow — warehouse-engine-swap

How to drive the stages in `template.yaml`. Read this before every
stage alongside `SCOPE.md` and the proposal at
[/home/user/code/rust/starter/rubix/docs/proposal/warehouse-engine-swap.md](/home/user/code/rust/starter/rubix/docs/proposal/warehouse-engine-swap.md).

## Sequencing

Three stages, no REVIEW gates. Strictly linear:

- Stage 2 (TimescaleDB impl) cannot start until stage 1's rename
  has landed and the `DdlDialect` seam exists — stage 2 adds a
  second impl behind that seam.
- Stage 3 (delete ClickHouse) cannot start until stage 2's
  TimescaleDB path is end-to-end green. Deleting the ClickHouse
  dialect before the TimescaleDB one works leaves the warehouse
  with zero working backends.

No REVIEW gates because the user wants this to run uninterrupted
for hours and the work is mechanical translation against an
already-reviewed proposal. If something genuinely surprising
surfaces (see "When to halt" below), halt and surface — do not
power through.

## Per-stage discipline

Before writing any code or docs in a stage:

1. Re-read the corresponding phase of the proposal. The proposal
   text is the contract; this WORKFLOW is the process.
2. Re-read `SCOPE.md` §"In scope" and §"Out of scope". The
   biggest risk on this job is creeping into deferred work
   (compressed-chunk runbook, hot-tag column promotion,
   MCP-only refactor). Stay strictly within the swap.
3. For stage 1: enumerate every `clickhouse` / `ChWriter` /
   `Ch*Reversible` reference in the repo before renaming. The
   rename is mechanical but must hit all sites in one stage —
   half-renamed code does not compile and will not pass
   `verify:`.
4. For stage 2: re-read the cagg constraints from the proposal
   (no subqueries / CTEs, single source hypertable, late-arrival
   tolerance, `tenant_id` in `GROUP BY`, `security_invoker =
   true`). Every cagg emitted by `DdlDialect::TimescaleDb`
   respects these.
5. For stage 3: grep the repo for `clickhouse` (case-insensitive)
   before declaring the stage done. Only the proposal doc itself
   should match.

Before committing a stage:

1. `cargo build --workspace` green from the starter repo root.
2. `cargo clippy --workspace --all-features -- -D warnings`
   green.
3. `cargo fmt --check` green.
4. `mani run build --all` green; `mani run lint` green.
5. For stage 1: the existing test suite still passes against the
   ClickHouse dialect (the rename and the `DdlDialect`
   extraction are behaviour-preserving).
6. For stage 2: TimescaleDB testcontainer smoke is green —
   ingest into all four hypertables, mart cagg refresh and
   query, retention drop, rule verb snapshot, undo path. Record
   the `cargo test -p starter-store-warehouse -- --ignored`
   transcript in the handover.
7. For stage 3: `mani run test --all` green; the cagg /
   retention / undo tests from stage 2 still pass; the repo
   grep for `clickhouse` (case-insensitive) outside the proposal
   doc returns no matches in live code.

Commit + push via **mani** from the codeless-workspace root:

```
./bin/mani --config mani.yaml run commit --projects starter \
  MSG='stage N: <one-line title from template.yaml>'
./bin/mani --config mani.yaml run push --projects starter
```

No `--force`, no `--no-verify`.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed. Do **not** rename or reorder them.

1. `checks` — run the stage's `verify:` list. Every step must
   pass. On failure: stop, fix, re-run; do not advance to
   `docs`.
2. `docs` — update `handover.md` for the next stage and tick
   the relevant `[x]` in `SCOPE.md` §"Deliverables".
3. `git` — stage the changes, commit with `stage N: <one-line
   title from template.yaml>`, push to
   `codeless/warehouse-engine-swap`. One stage, one commit.

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`.

## Anti-patterns specific to this job

- **Do not** add deprecated `rubix.clickhouse.*` aliases. The
  user explicitly opted out — old names are deleted outright.
  A "just in case" alias is wrong scope and wrong assumption
  (pre-production, no callers).
- **Do not** dual-write across both engines. Stage 2's
  TimescaleDB impl replaces the ClickHouse impl behind the
  `DdlDialect` seam; there is no parallel-running phase.
- **Do not** preserve `entities_dict` "for now". It is deleted
  in stage 2 when the mart queries that referenced it switch
  to direct JOINs. Leaving it as a dead table is a R12
  violation (no graveyard markers).
- **Do not** keep the `async_insert=1 / wait_for_async_insert=1`
  CI lint after stage 3. The discipline is engine-specific and
  no longer applicable; the lint becomes dead enforcement.
- **Do not** treat the proposal's Benchmarks section as gating.
  It is informational only — sizing + capacity planning.
  Missing a benchmark number is not a halt condition for this
  job.
- **Do not** ship a second `DdlDialect` impl in this job. The
  trait exists for future engine swaps; this job ships
  TimescaleDb only.
- **Do not** touch ADR-003 tenancy, `rubix.undo.last`, or the
  resolver-layer tenancy filter. They are explicitly preserved
  by the proposal's "what stays the same" list.
- **Do not** promote hot tag keys into columns or write a
  compressed-chunk backfill runbook. Both are explicitly
  deferred to post-launch in the proposal's risk table.
- **Do not** widen the rename. `warehouse` replaces
  `clickhouse`; it does not replace `timeseries`, `analytics`,
  or `olap` if any of those names exist elsewhere — they are
  distinct concepts (see the proposal's "rejected
  alternatives" note).

## When to halt

- A `MartSpec` definition in the existing codebase cannot be
  translated to a continuous aggregate (uses a subquery, CTE,
  or multi-source UNION). Halt at stage 1's audit step;
  resolution is to reshape the mart, which is a design
  question worth surfacing rather than silently rewriting.
- The `starter-store-postgres` testing seam cannot host the
  TimescaleDB extensions for the stage-2 smoke. Halt at
  stage 2; the resolution is in the testing seam (starter
  side), not in the warehouse crate.
- The `DdlDialect` extraction in stage 1 forces a wider
  refactor of `rubix_tools::warehouse::mart` than a single
  stage can absorb. Halt; the resolution is to split the
  extraction from the rename into two stages, which is a
  template change worth surfacing.
- Stage-3 repo grep for `clickhouse` finds matches the
  rename + delete missed. Do not paper over with a one-line
  fix in a "stage 3.5" commit — re-open the stage and clean
  the remaining references inside it, then re-run the trio.
