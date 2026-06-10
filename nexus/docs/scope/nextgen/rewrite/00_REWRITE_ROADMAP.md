# Nexus Rewrite — ArkFlow Removal + Data Engine Roadmap

> Verified: 2026-06-10 against master (6b6f16d2). Re-verify §0 before coding any RW.

## §1 Why (decision record)

ArkFlow is a single-maintainer project (1 human contributor, last release v0.5.0 Oct 2025,
recent activity ~74% dependabot). Nexus is pinned to an **unreleased git commit**
(`backend/Cargo.toml:63-64`, rev `b8f82b3`) because the `Stream::run(&mut self, token)`
cancellation signature exists only on HEAD, and carries a hand-trimmed **vendored fork**
of `arkflow-plugin` (`backend/vendor/arkflow-plugin/`, patch at `backend/Cargo.toml:120-121`).

Nexus uses only three things from ArkFlow: the `StreamConfig → Stream → run(token)` loop,
the builder registry, and the `sql`/`json_to_arrow` processors (thin wrappers over
DataFusion / arrow-json — both of which we keep as direct deps). Everything else
(all sinks, all sources, all runners, caps, stream multiplexing) is already nexus code.

**Decision:** remove ArkFlow. Replace the loop + registry with a native nexus engine core
(~500 lines), call DataFusion / arrow-json directly, then build the layers ArkFlow never
gave us: any-DB datasource sinks, cross-datasource federation, and a Polars + Rhai
insight stage. TimescaleDB/Postgres remains the store — **no second database**.

## §2 Target architecture

```
devices → MQTT broker (QoS = backpressure L1)
  → FlowManager: source → bounded channel (L2) → processors (DataFusion SQL, json_to_arrow)
                 → sinks (batched writes → ANY datasource by id)
  → store: pluggable datasource layer (Timescale default; Postgres/MySQL/files/…)
  → query dispatcher: push-down (native dialect, single datasource)
                      | federation (DataFusion TableProviders across datasources + files)
  → insights: nexus-insights crate — Polars (engine) + Rhai (sandboxed scripting)
  → serve: QueryRunner (JSON, caps) / LiveRunner (SSE) — unchanged public APIs
```

Extensions (WS-14 system, `nexus-api/src/extensions/`) are the plugin surface:
query-kinds exist already; RW-07 adds sources/sinks/insights contributions.
Rule: extensions contribute **into** the pipeline via host methods; they never bypass it.

## §3 Execution queue (dependency order — do not reorder)

| Order | RW | Title | Depends on |
|------:|----|-------|------------|
| 1 | RW-01 | Engine core: native pipeline loop, node traits, registry | — |
| 2 | RW-02 | Port nodes: sources/processors/sinks onto RW-01 (DataFusion direct) | RW-01 |
| 3 | RW-03 | Cutover: runners on the new engine; delete ArkFlow entirely | RW-02 |
| 4 | RW-04 | Any-DB store: sinks target a datasource id (datasource-kinds) | RW-03 |
| 5 | RW-05 | Federation: DataFusion across datasources + file/object-store kinds | RW-04 |
| 6 | RW-06 | nexus-insights: vectorized engine (DataFusion-first) + Rhai sandbox | RW-03 |
| 7 | RW-07 | Extension data-plane: contributes.sources/sinks/insights + ingest.write | RW-04, RW-06 |
| 8 | RW-08 | Backpressure hardening + soak test + flow metrics | RW-04 |

## §4 Owned files (lanes)

- RW-01: `nexus-engine/src/core/**` (new). May add `mod core;` to `lib.rs` (🔶 append-only).
- RW-02: `nexus-engine/src/{source,sink,processor}/**`, `arrow_json.rs`.
- RW-03: `nexus-engine/src/{runner,registry,flow}/**`, `nexus-engine/Cargo.toml`,
  `backend/Cargo.toml` (dep removal only), `backend/vendor/` (deletion),
  PLUS `nexus-engine/src/core/**` for the §6 contract alignment ONLY (lane explicitly
  transferred — RW-02 correctly refused it as out-of-lane; see TODOs.md "three core
  deltas still open": `&mut self`, `max_batch_rows` slice, `commit()` hook).
- RW-04: `nexus-engine/src/sink/datasource.rs` (new), `nexus-store/src/datasource/**`
  (append), `nexus-api/src/datasource_kinds/**` (append).
- RW-05: `nexus-engine/src/federation/**` (new), `nexus-api/src/routes/**/query*` (dispatch
  seam only), `nexus-api/datasource-kinds/` (new file kinds).
- RW-06: `crates/nexus-insights/**` (new crate), `nexus-spi` (new DTOs), query route
  insight param (🔶 small append), workspace `Cargo.toml` member append.
- RW-07: `nexus-api/src/extensions/**` (host_methods/contribute appends),
  `starter-ext-spi` manifest fields ONLY if additive + backward compatible.
- RW-08: `nexus-engine/src/flow/**` (metrics append), `backend/tests/soak/**` (new),
  docs `rewrite/BACKPRESSURE.md`.

🔶 Shared files (append-only, never restructure): `nexus-engine/src/lib.rs`,
`nexus-api/src/main.rs`/`state.rs`, workspace `Cargo.toml`s, `openapi.rs`.

## §5 Migration numbers

Reserve SQL migration blocks: RW-04 → `20xx`, RW-06 → `21xx`, RW-07 → `22xx`.
(`17xx`/`18xx` are TAKEN — `1701_nav_tree.sql` and `1801_extension_query_kinds.sql`
already exist in `nexus-store/migrations/nexus/`. Re-check the dir for the actual
latest before numbering; never reuse a block.)

## §6 Shared contracts

- **Engine node traits (RW-01, frozen after RW-02 starts):**
  `Source: async fn read(&mut self) -> Result<Option<RecordBatch>>` (None = finite end),
  `Processor: async fn process(&mut self, RecordBatch) -> Result<Vec<RecordBatch>>`
  (`&mut self` deliberately — stateful processors like windowing/dedupe must not need
  interior mutability; the pipeline applies processors sequentially so it costs nothing),
  `Sink: async fn write(&mut self, &RecordBatch) -> Result<()>` + `async fn close(&mut self)`.
  All config in/out as `serde_json::Value`. Registry: name → builder fn, same names ArkFlow
  used (`"sql"`, `"json_to_arrow"`, `"memory"`, `"generate"`, plus nexus customs) so stored
  flow configs in tenant DBs keep working **without migration**.
- **Batch size bound:** bounded channels bound *batch count*, not bytes — 64 × 100MB batches
  is an OOM with green metrics. The pipeline enforces `max_batch_rows` (config, default 8192)
  at the source and processor output boundary: oversized batches are sliced
  (`RecordBatch::slice` is zero-copy) before entering the channel. RW-08 soak-tests the
  fat-batch case explicitly.
- **Schema stability (json_to_arrow and all sources):** per-batch inference may NOT drift
  mid-stream. Contract: a flow either declares a schema in config (preferred for warehouse
  sinks) or the first batch's inferred schema becomes the stream schema and later batches
  are coerced to it; an incoercible batch is a source error (policy below), never a silent
  sink-side mutation.
- **Delivery semantics (ack/commit):** "no silent data loss" requires that a source MUST NOT
  acknowledge/commit upstream delivery (MQTT ack, queue commit) before the batch has been
  written by the sink. The trait carries `async fn commit(&mut self) -> EngineResult<()>`
  with a default no-op; the pipeline calls it after each successful sink write. Sources that
  cannot defer upstream acks (plain HTTP poll, simulator) keep the no-op and are documented
  at-most-once for in-flight batches; QoS-capable sources (MQTT, future queues) implement it
  for at-least-once.
- **Source error policy:** `read()` returning `Err` does not kill the flow by default —
  per-flow `source_on_error: retry_backoff (default, capped attempts) | halt`; exhausted
  retries → flow `last_error` state. Sink-side policy is RW-08's `on_error: halt|drop`,
  plus an optional dead-letter path (`on_error: dlq` → failed batches to the RW-04 file
  writer) once RW-04 lands — halt-vs-silent-drop is a brutal binary for a device fleet.
- **Pipeline run contract:** `Pipeline::run(CancellationToken)`; finite pipelines end when
  the source returns None and sinks have flushed; cancellation drains in-flight batch then
  closes sinks. Identical observable semantics to today's `stream.run(token)` paths.
- **Dependency versions:** RW-02/03 pin arrow/datafusion to ArkFlow's resolved versions for
  parity. **Post-RW-03 the pin is free:** bump arrow/datafusion to current before RW-05
  (federation leans on sqlparser fixes) — do not let the parity pin ossify.
- **Datasource sink contract (RW-04):** sink config = `{ "datasource": "<id>", "table": "…" }`;
  creds resolved via the existing envelope-encrypted store; writes batched (N rows or T ms).
- **Insight contract (RW-06):** `run_insight(script: &str, df: DataFrame, params: Value)
  -> Result<DataFrame>`; Rhai limits: max_operations, max call depth, wall-clock timeout,
  no file/network/module APIs registered.

## §7 Definition of Done (every RW)

- `cargo test --workspace` green; `cargo check --workspace` green at HEAD after commit.
- If DTOs changed: openapi regenerated + committed, `pnpm codegen`; if UI touched:
  `pnpm typecheck && pnpm test && pnpm build` green.
- Behavior parity proven where applicable: RW-02/03 must show existing engine tests
  (collector caps, SSE seq/resume, postgres sink, flows e2e) pass unmodified or with
  changes limited to import paths.
- Session log `rewrite/sessions/RW-xx.md` with Status/Started/Finished + commits.
- Final commit **pushed** to `origin nexus-rewrite` (the current branch — never a new
  branch, never force-push). Unpushed = not done.

## §8 Hard constraints

- **No second database.** Timescale/Postgres stays the store of record.
- **Public APIs frozen:** `QueryRunner::run`, `LiveRunner::spawn`, `FlowManager::start/stop`
  signatures and the HTTP/SSE wire contracts do not change (RW-06 adds optional fields only).
- **No new heavyweight default deps:** `rhai`, `object_store`, `parquet` are approved.
  `polars` is NOT pre-approved — it ships its own Arrow fork (polars-arrow/arrow2), meaning
  two Arrow stacks in one binary and a non-free RecordBatch↔DataFrame boundary; RW-06 must
  spike the insight primitives on DataFusion first and may adopt Polars only if ergonomics
  genuinely fail AND interop goes through the Arrow C data interface (zero-copy FFI), with
  the compile/binary cost recorded in the session log. Anything else heavy (duckdb, pyo3,
  librdkafka) is forbidden — that bloat is why the vendor trim existed. New connector-style
  deps must be feature-gated OFF by default (precedent: `mqtt`/rumqttc from WS-08b).
- ArkFlow code may not be copied verbatim (Apache-2.0 would allow it, but the point is a
  smaller, nexus-shaped core — write it fresh against the contracts in §6).
