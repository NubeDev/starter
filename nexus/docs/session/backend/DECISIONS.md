# Nexus Backend — Resolved Decisions

Decisions made during the autonomous backend build. Each is a one-liner with the
rationale that justified it. Newest first.

## D5 — R4 query-guard scope: read-only enforced server-side; shared-DB predicate deferred with config

The R4 guards split into what the control plane *enforces* and what the datasource *owner
configures*:

- **Enforced server-side, proven by test:** every datasource query runs in a `READ ONLY`
  Postgres transaction (a write/DDL is rejected by Postgres, not by string-matching — see the
  `read_only_guard_rejects_writes` test), with a `SET LOCAL statement_timeout` and a row/byte
  cap that stops the cursor (no unbounded buffer).
- **Datasource-owner's responsibility:** the *read-only DB role*. The control plane connects with
  the credentials the datasource config supplies; it cannot manufacture a least-privilege role
  inside someone else's database. The product guidance is that a datasource secret should be a
  read-only DB user; the `READ ONLY` transaction is defense-in-depth on top of that, so a
  misconfigured read-write user still cannot write through the query path.
- **Deferred (needs config, not in v1):** the per-tenant predicate for a datasource DB **shared
  across tenants**. v1 models one datasource as one tenant's data (the datasource row is
  tenant-owned and RLS-isolated in the metadata DB), so there is no shared-DB case to filter yet.
  Injecting a per-tenant `WHERE` requires a datasource-level tenant-column mapping in the
  datasource config — that lands when a shared-datasource connector does, not before. Building the
  predicate machinery now would be speculative config with no consumer.

## D4 — SQL datasource query path runs on sqlx, not ArkFlow's `sql` input

A consequence of D3: the connector trim removed ArkFlow's `sql` input (the one
piece that pulled DuckDB). So for SQL datasources the user's query executes
**directly against Postgres via sqlx**, and the result rows flow through the same
bounded-collector caps + Arrow/JSON shaping the ArkFlow seam uses. This is also the
*better* design independent of the trim:

- **R4 query safety is enforceable, not aspirational.** Owning the connection
  directly is how the read-only DB role, server-side statement timeout, forced
  `LIMIT`, and per-tenant predicate are actually guaranteed — they are connection-
  and-statement properties, awkward to enforce through an opaque ArkFlow input
  config.
- **Pushdown is automatic and total.** Running the user SQL as the input query
  against Postgres means `WHERE`/`LIMIT` execute in the database (the SCOPE's
  "two-layer SQL" resolution), with no in-memory full-table pull.
- **ArkFlow keeps its real job.** The collector/runner seam still drives the
  *live/streaming* path (memory/generate now; Kafka/MQTT/Modbus restored to the
  vendored plugin when M3 needs them, each pulling only its own dep). DataFusion
  remains available for non-SQL/cross-source shaping.

This narrows D1: ArkFlow is on the critical path for the *streaming* seam (proven at
M0), while the *one-shot SQL query* path is sqlx — the engine seam and the query
path are deliberately separate runners, as the SCOPE's two-runner model intends.

## D2 — R8: SSE auth = short-lived signed stream token in the URL (not cookie)

`POST /streams` (Bearer-authed) mints an HMAC-signed, ~60s-TTL token bound to the
stream registry key (spec + datasource + tenant + required permission); `GET
/streams/:id` reads it from the query string and verifies it. **Chosen over an
`HttpOnly` cookie** because the frontend is a separate-origin SPA already on Bearer
for REST — a cookie path drags in CSRF + CORS-credentials handling for one route,
while a per-subscription signed token is least-privilege (it authorizes exactly one
stream, not the whole session) and needs no extra browser state. Native `EventSource`
can't set headers, and the token-in-URL is acceptable because the token is
short-lived, single-audience, and carries no standing credential.

## D3 — ArkFlow build weight (Risk #4): vendor a connector-trimmed arkflow-plugin

`arkflow-plugin` registers every connector unconditionally and has **no feature
gates**, so depending on it for the `sql`/`memory`/`json` builders the seam needs
also drags in DuckDB (a ~15 GB static C++ build that exhausted the disk), librdkafka
(needs system `curl/curl.h`), PyO3, Ballista, Pulsar, NATS, Redis, Modbus, and more.
The `sql` input even `use`s `duckdb::` directly, so cargo features alone can't drop
it.

Resolved by vendoring a **connector-trimmed copy** of `arkflow-plugin` under
`nexus/backend/vendor/arkflow-plugin` and redirecting the upstream git dep to it
with a workspace `[patch]`. The copy keeps only the pure-DataFusion modules
(`memory`/`generate` inputs, `sql`/`json_to_arrow` processors, `drop`/`stdout`
outputs, and their support modules) and a slim manifest; every native connector
and its dependency is removed. **`arkflow-core` (the engine) is consumed unpatched**
— this trims connector *selection*, it does not fork the engine, so R3 holds. Build
dropped from minutes to ~45 s and disk from 184 GB used to ~114 GB; no curl/cmake/
duckdb toolchain is needed. Re-sync the vendored copy against the pinned upstream
rev on every ArkFlow bump (its provenance is recorded in its `src/lib.rs`). The
right permanent fix is upstream feature-gating; until then this patch is the cost.

## D1 — Risk #17: ArkFlow is on the M0 critical path (option a), not deferred to M3

The standalone POC (`nexus/poc/backend`, arkflow rev `b8f82b3`) already proves the
Collector-sink → `Stream::run(token)` → Arrow→JSON seam end-to-end — the single
biggest ArkFlow risk (Risk #1, request/response-over-streaming) is **already
retired**. Re-targeting M0–M2 onto raw DataFusion+sqlx and swapping ArkFlow back in at
M3 would mean writing the query path twice and discarding a working seam. The git-rev
cancellation API (Risk #5) is likewise confirmed present at that rev. So M0 builds the
real ArkFlow seam now; the DataFusion+sqlx fallback stays unused unless a later
ArkFlow bump breaks the pinned signatures.
