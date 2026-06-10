# Nexus Rewrite — TODOs / Blockers

> Sessions append here when blocked (per the AGENT CHARTER no-questions rule) or when deferring
> a follow-up. The human resolves blockers by adding a `✅ RESOLVED:` line under the entry;
> the loop then resets the blocked row to ⬜.

Format per entry:

```
## YYYY-MM-DD <RW-xx> — <one-line title>
- **Type:** blocker | follow-up
- **What:** <what is needed / what was deferred>
- **Why:** <why the session could not proceed / why deferred>
- **Proposed:** <the session's recommended resolution>
```

---

## 2026-06-10 RW-01 — Peer-review contract updates landed mid-flight (reconcile at gate / RW-02)
- **Type:** follow-up
- **What:** Human peer review updated roadmap §6 + RW-01/02/04/05/06/08 specs WHILE RW-01's
  subagent was already building. Deltas affecting RW-01's lane:
  (1) `Processor::process` is now `&mut self` (was `&self`),
  (2) new `max_batch_rows` slicing contract at source/processor output boundary,
  (3) single-output config shape needs the one-grep fan-out check before freeze,
  (4) [codex review] `Source::commit()` default-no-op ack hook — the pipeline calls it
      after each successful sink write (§6 delivery semantics; MQTT implements it later).
  Unaffected-but-frozen-later: schema stability rule (RW-02), source_on_error policy (RW-08).
- **Why:** §6 freezes when RW-02 starts; cheaper to align now than after the freeze.
- **Proposed:** Whoever gates RW-01: if it shipped `&self` or lacks the batch bound, do NOT
  fail the gate — spawn the same-charter fix pass (gate step 4 mechanism) or fold the
  alignment into RW-02's first action (it re-reads §6 anyway). Both are small mechanical
  changes while core/ has a single consumer.

## 2026-06-10 RW-02 — Built against RW-01's as-committed contract; three core deltas still open
- **Type:** follow-up
- **What:** The reconciliation above was NOT applied to `core/` before RW-02 ran. RW-02 ported
  every node against the committed `core::node.rs`: `Processor::process(&self)`, no
  `Source::commit()`, no `max_batch_rows` slicing in `core::pipeline.rs`. All three of those
  live in RW-01's lane (`core/**`), which RW-02 must not restructure (ROADMAP §4), so they
  remain open:
  (1) `&self` vs `&mut self` — RW-02's processors are `&self`; harmless to flip to `&mut self`
      later (none need shared access), but it is an RW-01 trait change.
  (2) `Source::commit()` ack hook — absent. RW-02's ports are pull-only (memory/generate/http
      poll/simulator) and need no ack; MQTT (later) is the first that does.
  (3) `max_batch_rows` source/processor-output slicing — absent from the pipeline. RW-02's
      sources emit small batches (1 doc, or batch_size small docs), so no OOM risk yet; the
      fat-batch case is RW-08's soak test, which needs the pipeline-side slice to exist.
- **Why:** These are `core/` (RW-01) changes; RW-02 stays in its `{source,sink,processor}` lane.
- **Proposed:** RW-03 (next to touch `core/`-adjacent runner wiring) or an RW-01 fix pass:
  add the slice + commit hook to `core::pipeline.rs` and flip the trait to `&mut self`. RW-02's
  nodes compile unchanged under `&mut self` and gain a default-no-op `commit()`.

## 2026-06-10 RW-04 — `datasource` sink has no flow-builder palette descriptor (RW-03 lane)
- **Type:** follow-up
- **What:** The new `datasource` output sink is registered and runs end-to-end, but it has
  no entry in `nexus-engine/src/registry/descriptor.rs::describe()`, so `GET
  /api/v1/flows/node-types` does not surface it for the visual flow builder. A user can
  still author a datasource-targeted flow via raw config (`{type:datasource, datasource:id,
  table}`), which the start handler resolves; only the palette is missing.
- **Why:** `registry/**` is RW-03's lane (ROADMAP §4); RW-04 owns `sink/datasource.rs`,
  not the descriptor table, and the charter forbids editing another RW's lane. The feature
  is fully functional without the palette entry, so this is a usability follow-up, not a
  blocker.
- **Proposed:** Whoever next touches `registry/descriptor.rs` (an RW-03 fix pass, or RW-07
  when it adds extension-contributed sinks to the palette) adds a `datasource()`
  descriptor: category Output, config_schema `{kind, datasource(id), table, batch_rows?,
  batch_ms?}`, and extends the `describes_every_registered_node` test.

## 2026-06-10 RW-05 — File datasource (parquet/csv) cannot be *persisted* — store schema is Postgres-shaped
- **Type:** follow-up
- **What:** The `parquet`/`csv` datasource kinds are declared (manifest pack) and the engine
  reads them natively (`FederatedSource::{Parquet,Csv}`, engine test green), but a file
  datasource cannot be stored as a `nexus_datasources` row. Migration 0001's table is rigidly
  Postgres-shaped — `host`/`port`/`database`/`db_user` NOT NULL, `secret_cipher` NOT NULL —
  with no generic `config`/`path` column and no way to omit the secret. So
  `federation::resolve::resolve_one` returns `Invalid` for a stored file kind today: it can
  authorise + decrypt a `postgres` source, but a file source has no record to resolve.
  Postgres↔Postgres federation is therefore fully wired end-to-end; file *persistence* is the
  one missing leg of an end-to-end docker-PG ⋈ stored-Parquet join.
- **Why:** The fix is a store-side migration (a nullable `config jsonb` / `path` column +
  nullable secret columns) plus `record.rs`/`insert.rs`/`get`/`resolve` changes — squarely
  RW-04's `nexus-store/src/datasource/**` lane and RW-04's 20xx migration block. The charter
  forbids editing another RW's lane, and a second DB / schema change is out of RW-05's scope.
- **Proposed:** An RW-04 fix pass adds a nullable `config jsonb` column (carrying `{path,
  has_header}` for file kinds) and makes the secret columns nullable for secret-less kinds;
  then `resolve::resolve_one`'s `parquet`/`csv` arms build `FederatedSource::{Parquet,Csv}`
  from `record.config.path` (engine + manifest already support it). Until then the file arms
  stay an explicit `Invalid` (loud, never a silent drop), and the E2E join is proven with two
  registered Postgres datasources (`federation_e2e_test.rs`).

## 2026-06-10 RW-02 — Native `sql` omits ArkFlow's JSON UDFs (confirm before vendor delete)
- **Type:** follow-up
- **What:** ArkFlow's vendored `sql` processor registers `datafusion_functions_json` + a custom
  `udf::init` set on its SessionContext. The native `processor/sql.rs` uses a plain
  `SessionContext` (no JSON UDFs) — no stored flow / existing test uses them, and §8 has not
  approved a JSON-UDF dep.
- **Why:** Adding the dep speculatively would violate §8 (no unapproved heavy deps); dropping it
  silently could break a tenant flow that used `json_get(...)` in its SQL.
- **Proposed:** RW-03, before deleting `vendor/arkflow-plugin/src/processor/sql.rs`: grep stored
  tenant flow configs for JSON-UDF usage in `sql.query`. If any exist, raise a blocker to get
  `datafusion-functions-json` approved as a direct dep; otherwise the omission is safe.
