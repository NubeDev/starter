# Nexus Backend — Resolved Decisions

Decisions made during the autonomous backend build. Each is a one-liner with the
rationale that justified it. Newest first.

## D9 — Alerting is a scheduler + state machine + notifiers, designed in ALERTING.md before any code

The alerting subsystem follows its own sub-design ([ALERTING.md](ALERTING.md)), written before
implementation per the session scope. The load-bearing decisions:

- **The state machine is the dedup.** A pure `step(state, breaching, dwell_elapsed)` function
  (ok→pending→firing→resolved) emits a transition only on `→firing`/`→resolved`, so a rule
  breaching for an hour notifies once, not every tick. It has no I/O and is exhaustively
  unit-tested; the evaluator wraps it with persistence and notification. The `for_secs` dwell
  routes a fresh breach through `pending` first, absorbing a transient spike.
- **The scheduler's cross-tenant discovery is a SECURITY DEFINER function, not BYPASSRLS.** A
  system task must find due rules across every tenant, which RLS forbids the runtime role from
  doing. Rather than weaken the runtime role, `nexus_claim_due_alert_rules` (owned by the
  migration role, `EXECUTE`-granted to the runtime role) exposes exactly that one cross-tenant
  read, advances `next_eval_at` atomically, and uses `FOR UPDATE SKIP LOCKED` so the single-node
  v1 upgrades to multi-node without a claim rewrite. Each claimed rule is then evaluated under
  its own tenant's RLS context.
- **Silences suppress notification, never evaluation.** A silenced rule still evaluates and still
  writes its event (the history stays honest); only the channel delivery is skipped, with the
  event flagged `silenced`. This is the maintenance-window path.
- **Channels are a trait + kind enum; v1 ships webhook.** Webhook is the universal integration
  (Slack/PagerDuty/email gateways all accept one) and needs no provider SDK, so it is the only
  kind built; `email`/`slack` are a new arm + impl, not an evaluator change. A channel failure is
  recorded on the event and never crashes the evaluator or blocks the other channels.
- **The evaluator lives in `nexus-api`, not `nexus-engine` (R2).** It orchestrates the store and
  the existing guarded query path; putting it in the engine would force the store into the engine.
  The pure pieces (state machine, threshold comparison) are unit-tested; the evaluator is proven
  end-to-end (rule fires through a real webhook, dedups while firing, resolves on recovery).

Deferred, with the upgrade path noted: multi-node evaluation (claim already SKIP-LOCKED-safe),
conditions beyond single-scalar-vs-threshold (operator enum is add-only), channel kinds beyond
webhook, and durable notification retry (v1 records the failure on the event).

## D8 — Flow connectors are nexus custom builders (http_poll input, postgres output), not a vendor restore

FlowManager runs the topology's weather→Postgres ingestion, which needs an input that
*polls* an upstream API and an output that *writes* to Postgres. The trimmed vendor
(D3) has neither. Two ways to get them; the chosen one avoids touching the vendor:

- **`http_poll` input + `postgres` output are registered ArkFlow builders living in
  `nexus-engine`** (`source/http_poll.rs`, `sink/postgres.rs`), mirroring the existing
  `collector`/`sse` custom outputs. This is the SCOPE's "extend data sources via
  `register_input_builder`" path — no core change, no fork, and crucially no vendor
  edit. The alternative — restoring upstream's `http` input and `sql` output — was
  rejected because upstream's `http` input is an *ingress HTTP server*, not a poller
  (wrong shape for "poll weather every 15m"), and would drag axum/tower/flume/subtle
  into the vendor for a tool that does not even fit the use case. `sqlx` and `reqwest`
  (already workspace deps) are the only additions, both to `nexus-engine`.
- **Kafka and Modbus stay out**, per the session scope. The only connectors restored
  are the two the documented flow needs, and they are added as nexus code, not vendor.

A flow is config-not-code: the `flows` table stores its three ArkFlow config blobs
(input/pipeline/output) as jsonb, tenant-scoped and RLS-isolated like datasources.
`FlowManager` runs each as a long-lived `Stream` keyed by the immutable flow id, with
idempotent `start` and a `stop` that cancels the token; the running set is in-process
(single-node for v1, like live fan-out — a multi-node flow scheduler/leader-election is
a later concern, stated not discovered). The REST surface gates on the `nexus.flow`
grant kind (D6): view→get, edit→update/start/stop, delete→delete; start also flips the
stored `enabled` flag so the intent survives for a future resume-on-boot. End-to-end
test: a flow created over the API polls a real local endpoint and lands the response in
a Postgres table, then stops.

## D7 — Live SQL panels are a poll loop over the guarded query, not an ArkFlow streaming input

A live panel watches a SQL datasource, but the engine has no streaming way to do
that: the connector trim (D3) removed ArkFlow's `sql` input (it pulled DuckDB), and
D4 already moved the one-shot SQL path onto sqlx-direct for enforceable R4 guards.
There is no push source for "rows changed in Postgres" to subscribe to. So a live SQL
panel is modelled as a **poll loop**: a new `PollRunner` in `nexus-engine` re-runs a
caller-supplied producer on an interval and publishes each result to the run id's
broadcast channel — the same channel the SSE sink and subscribers already use. The
producer is injected by `nexus-api` (it calls `nexus_store::run_query`, so the live
path inherits the *exact* read-only/timeout/row-cap guards the one-shot path has), so
`nexus-engine` keeps no DB dependency and owns only cadence + publish.

The ArkFlow `generate`/SSE seam is **not** removed — it remains the path for genuine
push sources (a future restored MQTT/Kafka input drives the SSE sink directly via
`LiveRunner`). Poll covers SQL; push covers brokers; both share the broadcast +
stream-registry + signed-token machinery.

Wiring: `POST /streams` is now Bearer-authed (it runs behind the principal layer),
checks the datasource is visible to the tenant and the caller may `view` it (the same
grant gate as the REST handlers, D6), parks the vetted SQL in an in-process registry
keyed by the new stream id, and mints a token bound to the caller's **real** tenant +
permission (no more hardcoded `"dev"`/`"view"`). `GET /streams/:id` verifies the
token, and on the first subscriber consumes the parked spec to start the poll;
later subscribers of the same id share the running loop; the last to leave tears it
down (refcount → cancel, unchanged). The parked spec is in-process because live
fan-out is single-node for v1 (R7) — a subscription only lands on the node that
minted its token — and it expires on the token's own TTL so an abandoned create
cannot leak. Per-datasource connection (vs the single dev `state.datasource` pool)
remains the same noted follow-up the one-shot `/query` path carries; it is not unique
to live and is deferred with it, not introduced half-done here.

## D6 — Authz: per-instance grants enforced in handlers; engine fixed upstream; admins pass by role

The M2+ follow-up — "per-resource view/edit/manage grants on immutable ids, RLS as
defense-in-depth" — is now enforced in the dashboard/datasource handlers. Three sub-decisions:

- **The policy engine had to learn per-instance scoping.** `starter-authz` already wrote a
  `resource_id` to each grant row, but the *engine ignored it* — `config::Rule` carried no
  `resource_id`, so every rule matched its whole kind and a grant on one dashboard authorized
  all of them. Per the SCOPE rule "fix it upstream and consume it, don't grow a parallel crate
  in nexus", the fix lands in `starter-authz`: thread `resource_id` through `config::Rule` →
  `CompiledRule` and add an instance match in `check()` (`None`/`"*"` stays kind-wide, a
  concrete id must equal `object.id`). Strictly additive — every existing rule has no
  `resource_id` and keeps its kind-wide behaviour. This makes per-immutable-id grants real for
  the whole platform (rubix included), not just nexus.

- **`default_policy` stays `true` (built-in role ladder kept), not flipped to default-deny.**
  Flipping to default-deny would force even a tenant admin to hold an explicit grant for every
  resource. Instead the built-in `admin → */*` rule lets a tenant admin reach their tenant's
  resources by role, while non-admins match no built-in rule on the nexus action vocabulary
  (`view`/`edit`/`delete`) and so get access *only* from explicit per-resource grants — which is
  exactly the sharing model the product wants. The engine composes the two layers natively
  (allow-if-any-allow, deny-overrides); the tenant-scoping predicate isolates either way.

- **The shared engine is one instance, not two.** `identity::build` constructs the
  `DbPolicyEngine` once and hands the same `Arc` to both the `/v1/authz/*` router (which calls
  `reload()` after a grant write) and `AppState` (which calls `check()` in handlers, via a
  `dyn PolicyEngine` upcast). A grant created over the API is therefore visible to the next
  handler check with no second handle to keep in sync. Tests swap an `AllowAll`/`DenyAll`
  engine into `AppState` to assert a route is gated independent of any seeded policy.

Grant checks key on the resource's immutable id (a dashboard slug is resolved to its id first),
run *after* the existence check so a hidden row is a 404 and a forbidden one a 403, and sit on
top of RLS — RLS hides other tenants' rows, the grant gates what the tenant's own members may do
to a row they can see.

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
