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
- **✅ RESOLVED (RW-04b):** migration `2001_datasource_file_config.sql` adds the nullable `config
  jsonb` + nullable secret/connection columns; `record/insert/fetch` carry `config` and an optional
  secret; `federation::resolve::resolve_one`'s parquet/csv arms build `FederatedSource::{Parquet,
  Csv}` from `record.config.path`. New `stored_parquet_joins_live_postgres_end_to_end` e2e proves
  the stored-Parquet ⋈ live-PG join.

## 2026-06-10 RW-09 — `ingest.write` should route through the engine's push channel (RW-07b)
- **Type:** follow-up
- **What:** RW-09 built the bounded push-ingest path self-contained in the engine
  (`source/http_ingest.rs` → `IngestChannels` on `FlowManager`, non-blocking
  `try_push` with `429 + Retry-After` on a full channel). The spec said to *share*
  this with RW-07's `ingest.write`, but RW-07's data-plane (items 2–4, incl.
  `ingest.write`) is deferred to RW-07b, so there was nothing to share *into* yet.
- **Why:** Building `ingest.write` here would have crossed into the deferred
  extension data-plane lane (two-workspace host-method + supervisor wiring).
- **Proposed:** RW-07b's `ingest.write` host method should resolve the named flow's
  sender via `state.flows.ingest()` and call `IngestChannels::try_push`, returning
  `retry_after` on `IngestError::Full` — reusing RW-09's channel/backpressure seam
  rather than introducing a second path. The host stamps tenant from the install
  identity (never the payload), exactly as the spec requires.
- **✅ RESOLVED (RW-07b):** `nexus-api/src/extensions/ingest.rs::write` resolves the
  named source via `state.flows.ingest()` and calls `IngestChannels::try_push`,
  returning `retry_after_secs` on `IngestError::Full` — exactly the RW-09 seam, no
  second path. Tenant stamped from `caller.tenant_id` (the supervisor binds it from
  the install), overwriting any payload `tenant_id`.

## 2026-06-10 RW-09 — Zenoh store-side connect probe not implemented (RW-04 lane)
- **Type:** follow-up
- **What:** The `zenoh` datasource kind is declared in the manifest pack
  (`datasource-kinds/zenoh_config.json` + `manifest.yaml`, Stream surface, no
  secrets) and the engine reads it natively behind the `zenoh` feature, but there
  is no store-side connect/probe — `test_connection` has no zenoh arm. This mirrors
  `mqtt`, which is also catalogue-only with no `test_connection` enum wiring.
- **Why:** A store-side connect probe lives in RW-04's `DatasourceKind` /
  `test_connection` lane; the charter forbids editing another RW's lane. The
  catalogue entry is fully functional for flow authoring without it.
- **Proposed:** Whoever extends `test_connection` (an RW-04 fix pass) adds a zenoh
  arm that opens a short-lived session against the configured endpoints and reports
  reachability, the same shape as a future mqtt probe.
- **✅ RESOLVED (RW-04b):** `DatasourceKind` gains `Mqtt`+`Zenoh`; `test_connection` dispatches a
  zenoh arm (and an mqtt-parity arm) reading params from the request `config`. New feature-gated
  store probe `datasource/zenoh/probe.rs` (`zenoh` feature, OFF by default, mirrors `mqtt`) opens
  a short-lived `zenoh::open` against the endpoints and reports reachability; feature-off returns
  a clear "not enabled" error. Zero zenoh deps in a default build.

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

## 2026-06-10 RW-06 — Two pre-existing nexus-api test binaries fail to compile (stale drift)
- **Type:** follow-up
- **What:** `tests/routes/authz/grant_gate_test.rs` constructs `NewDashboard` without the
  `icon`/`accent`/`folder_id` fields it gained in a later RW, and
  `tests/routes/identity/wiring_test.rs` calls `serve::assemble` with 5 args after it grew a
  6th (`Router<AppState>`). Both fail `cargo test -p nexus-api --no-run`. These files are
  outside RW-06's lane and were already broken on `nexus-rewrite` before this session — the
  RW-06 DTO change (additive `insight` field on `QueryRequest`) does not touch them.
- **Why:** Fixing them means editing another RW's test lane; RW-06 must stay in-lane. They do
  not block RW-06's own test binaries (`routes_insights_e2e`, `routes_query_insight_e2e`),
  which compile clean.
- **Proposed:** The RW that owns the dashboard-DTO / `serve::assemble` change (or a dedicated
  drift-fix pass) updates these two call sites: add the three `NewDashboard` fields and the
  missing `assemble` router argument. Until then they are stale, not regressions.

## 2026-06-10 RW-07 — Extension data-plane sources/sinks (`ingest.*`) deferred — only the insights slice shipped
- **Type:** follow-up (RW-07 scope items 2, 3, 4)
- **What:** RW-07 shipped only its insights slice (spec items 1 + 5): `contributes.insights[]`
  boot lint+materialise+cleanup into the global `nexus_extension_insights` table, the
  `InsightRef.insight_name` query path, and the `com.nexus.hello.zscore` demo. The larger
  host-mediated data-plane — spec items 2–4 — is NOT done:
    - `contributes.sources[]` / `contributes.sinks[]` manifest fields
      (`{name, config_schema, direction}`) in `starter-ext-spi/src/manifest.rs` (additive).
    - host method `ingest.write` (extension pushes JSON rows tagged with a registered source
      name; host stamps tenant from the extension's *install identity*, NEVER the payload;
      json_to_arrow → the named flow source's bounded channel; returns `retry_after` when the
      channel is full — document the backpressure contract in the SPI).
    - sink direction: `ingest.read_batch` (long-poll) OR push via supervisor JSON-RPC — pick
      ONE based on what `starter-ext-supervisor` already supports best; document the choice.
    - engine `source/extension.rs` + `sink/extension.rs` nodes, registered under contributed
      names at extension boot and deregistered on disable/purge; a flow referencing a missing
      extension node must fail to build with a clear error (test).
    - authz: gate the `ingest.*` host-method categories the same way `warehouse` is gated (see
      the kernel category-gate comment in `nexus-api/src/extensions/host_methods.rs`).
    - migration `22xx` only if registration state needs persistence beyond existing extension
      tables (`2201_extension_insights.sql` is already taken by the insights slice; use `2202+`).
  Outstanding acceptance bullets: a process-runtime test extension pushing rows through
  `ingest.write` into a flow that lands them in a datasource sink (docker-gated e2e, tenant
  stamped by host + verified); the channel-full backpressure response (test with a tiny channel
  capacity); and extending the hello e2e to assert `DELETE …?purge=true` removes the insight
  via `InsightCleanupProvider` (the cleanup path exists and is unit-covered, but the hello e2e
  was not extended with a new purge assertion this pass).
- **Why:** Items 2–4 are a substantially larger cross-workspace data-plane (starter-extensions
  host methods + supervisor wiring + engine nodes + authz). The spec itself names item 1 "the
  smallest slice that proves the pattern"; shipping 2–4 half-done would mean stubs across two
  workspaces. The insights slice is self-contained, fully tested, and unblocks downstream RWs
  that only need the contributed-insight path.
- **Proposed:** A dedicated RW-07b (or reopened RW-07) implements the sources/sinks data-plane
  against the host_methods.rs category-gate pattern and the supervisor JSON-RPC the spec points
  at, mirroring the channel/backpressure design in the engine's existing bounded-source path.
  Additive manifest fields only — if any needed `starter-ext-spi`/supervisor change is breaking,
  that is a blocker entry here, not a guess (per the spec's Non-goals).
- **✅ RESOLVED (RW-07b):** items 2 (sources/sinks manifest fields + `ingest.write`), 3 (engine
  source seam), and 4 (`ingest.*` authz) shipped. Additive contracts:
  `Contributes.sources[]`/`sinks[]` + `ContributeSource`/`ContributeSink`/`IngestDirection`,
  `Capability::Ingest { names }`, and the `ingest::{IngestWriteRequest,IngestWriteResponse,
  IngestReadBatchRequest,IngestReadBatchResponse}` DTOs in `starter-ext-spi`. Supervisor gate:
  `("ingest","ingest")` in `CAPABILITY_HOST_METHODS` + `Capability::Ingest => "ingest"` in
  `category_of`; host `capability_matches` + wasm `from_capability` arms added (additive). Host
  method `ingest.write` in `nexus-api/src/extensions/ingest.rs` (tenant-stamp from caller,
  backpressure via the RW-09 `IngestChannels` seam). The engine "extension source node" is the
  existing per-flow `http_ingest` source keyed by flow id (RW-09) — a stopped flow's source
  deregisters its channel, so a push afterward returns `NotRunning` (the "errors cleanly after
  purge/disable" acceptance). Acceptance bullets closed: channel-full retry_after unit test,
  tenant-stamp docker e2e, and the hello-purge docker e2e (`InsightCleanupProvider`).
  **Still open:** the *sink* direction `ingest.read_batch` (host→extension drain) — see the
  RW-07b follow-up below.

## 2026-06-10 RW-07b — Sink direction `ingest.read_batch` (host→extension drain) deferred
- **Type:** follow-up (RW-07 scope item 2, sink half)
- **What:** The data-plane *source* direction (`ingest.write`) is fully shipped. The *sink*
  direction — a flow sink whose batches an extension long-polls via `ingest.read_batch` — is
  NOT implemented. Its additive contracts ARE landed (`ContributeSink`, `IngestReadBatch{Request,
  Response}`, the `ingest` capability category), so the remaining work is purely engine-side:
    - an engine `nexus-engine/src/sink/extension.rs` node that writes each batch into a bounded
      per-sink output queue (the symmetric dual of `IngestChannels`, e.g. `IngestOutputs` on
      `FlowManager`), back-pressuring the flow when the extension is not draining;
    - a `nexus-api` `ingest.read_batch` host method that long-polls that queue under the caller's
      capability/tenant gate and returns up to `max_rows` rows.
- **Why:** Unlike `ingest.write` (which reuses RW-09's existing `http_ingest` push channel), the
  sink drain has no existing seam to reuse — it is a second self-contained data-plane (engine
  queue + flow wiring + long-poll method). Shipping it half-done would mean a stub host method
  that pretends to drain. Per the charter (no stubs in shipped paths) it is deferred whole.
- **Proposed:** A focused RW-07c (or reopened RW-07b) adds the `IngestOutputs` queue +
  `sink/extension.rs` + `ingest.read_batch` against the already-landed contracts, mirroring the
  `IngestChannels`/`http_ingest`/`ingest.write` shape exactly. No new SPI/supervisor change is
  needed — the contracts are in place.

---

## 2026-06-10 RW-08 — pre-existing test drift under `--features testing` (out of lane)

Two nexus-api test binaries fail to compile **only** with `--features testing` (the
default `cargo test --workspace` is green because both files are
`#![cfg(feature = "testing")]` and compile empty without it):

- `crates/nexus-api/tests/routes/identity/wiring_test.rs:61` — `serve::assemble` now
  takes 6 args (a `Router` was added); the call passes 5.
- `crates/nexus-api/tests/routes/authz/grant_gate_test.rs:88,117` —
  `NewDashboard` gained `accent`, `folder_id`, `icon`; the two literals omit them.

Neither file is in RW-08's lane (flow metrics / soak / BACKPRESSURE.md) and the
drift predates this session (dashboard-folder + identity-assemble work added the
fields/arg). RW-08 left them untouched. **Action:** the owning lane (nav/dashboard
+ identity) should refresh these two literals/call-sites; they are a one-line fix
each. Until then `cargo clippy --all-targets --features testing` on nexus-api is
red on these two targets only.
