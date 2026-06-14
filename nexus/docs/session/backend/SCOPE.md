# Nexus Backend — Scope (`nexus/backend/`)

> **Before you write code:** read [../README.md](../README.md) — the session coding rules
> (one responsibility per file; comments say *why*, never the stage/fix/phase/rule that
> produced them; planning docs stay out of source).
>
> **Architecture source of truth:** [`../../scope/NEXUS.md`](../../scope/NEXUS.md) +
> [`../../scope/NEXUS_TOPOLOGY.md`](../../scope/NEXUS_TOPOLOGY.md). This doc is the **build scope** —
> what we ship, the crate/file layout, the load-bearing rules, the phases, and the
> **subagent work-units**. Frontend scope is the sibling [`../ui/SCOPE.md`](../ui/SCOPE.md).
>
> **Layout law:** every rule about files here is downstream of
> [`../../../../rubix/FILE-LAYOUT.md`](../../../../rubix/FILE-LAYOUT.md). One verb per file,
> folder-of-verbs over file-of-nouns, ≤400 lines hard / ~100 typical, names are concepts
> (never `utils`/`helpers`/`common`).

## One-line summary

`nexus-api` is the **Rust/Axum control plane** that turns the **ArkFlow** stream engine into a
multi-tenant BI/observability product: auth + teams (from `starter-auth-users` /
`starter-authz`), datasources, dashboards/panels, **one-shot queries** (Collector sink), **live
panels** (SSE sink), saved **flows** (light ingestion), and alerts — over REST + SSE, on
Postgres with RLS.

## The mental model

**ArkFlow owns the engine; nexus-api owns the product.** ArkFlow gives us connectors +
DataFusion SQL + a public plugin registry + Arrow streaming, and *nothing else* (its only HTTP
is a health check). Everything a product needs — identity, persistence, a REST/SSE API, query
governance, tenancy — is the control plane's job. Direction is one-way: **UI → nexus-api →
ArkFlow**, never the reverse. Identity is owned by the `starter-*` crates, not reinvented here.

The bridge from "streaming engine" to "request/response + live product" is **two custom ArkFlow
output sinks** (Collector, SSE) driven by **two runners** (Query, Live). That seam is the whole
backend's keystone — M0 exists to prove it.

## What nexus-api is, exactly

A single backend that exposes, all under `/api/v1` (auth routes keep their own `/auth/*` mount):

1. **Identity passthrough** — mounts `starter-auth-users` (`/auth/login|token|logout|me|signup`)
   and `starter-authz` (`/authz/grants`) routers unchanged; adds **`GET /api/v1/me`** to fill
   the `/auth/me` gap (`tenant_id` + teams + effective permissions).
2. **Datasources** — CRUD + `test` + `query`. A datasource = (ArkFlow input `type` + saved,
   **encrypted** connection config + tenant owner).
3. **Query** — `POST /query`, `POST /datasources/:id/query`: one-shot Stream → Collector sink →
   Arrow→JSON, under the §R4 query guards.
4. **Live (SSE)** — `POST /streams`, `GET /streams/:id` (subscribe), `DELETE /streams/:id`:
   unbounded Stream → SSE sink → broadcast → `EventSource`/fetch-SSE.
5. **Dashboards / panels** — CRUD; grants and panel refs key on the **immutable id**, slug is a
   route alias.
6. **Flows** — saved ArkFlow Streams the `FlowManager` runs (light ingestion, e.g.
   weather→Postgres). In v1; heavy ETL/CDC is a non-goal.
7. **Alerts** — rules + events surface. **The evaluator is its own sub-design** (scheduler,
   state machine, channels) — not just CRUD; see §Phases M3.
8. **Plugin registry** — upstream ArkFlow input/output plugins + **our** Collector/SSE outputs
   and any custom inputs, via `register_*_builder()`.

Anything not on that list is out (§Non-goals).

## Hard rules (load-bearing) — R1…R12

Headers are authoritative; "R*n*" in any backend doc/comment means the rule with that header.

### R1 — One responsibility per file. 400 lines. Always.
Per [FILE-LAYOUT.md](../../../../rubix/FILE-LAYOUT.md): ≤400 lines hard, ~100 typical; ≤50 lines per
function; ~10 public items per module; nesting ≤4. Name files after the concept
(`collector.rs`, `arrow_json.rs`), never `utils.rs`/`helpers.rs`/`common.rs`/`types.rs`. SQL
migrations and codegen are exempt (under `migrations/` / `generated/`).

### R2 — Layer arrow: `nexus-spi → {nexus-engine, nexus-store} → nexus-api`.
Contracts depend only on `starter-spi`. Engine and store depend on `nexus-spi` + their upstream
(ArkFlow / sqlx). The **binary** (`nexus-api`) depends on all and wires transports. Never the
reverse; `nexus-spi` has zero internal deps.

### R3 — Depend on ArkFlow, never fork it.
Pull `arkflow-core` + `arkflow-plugin` as a **pinned git rev** (cargo `git =` + `rev`). Drive
`Stream` directly; **never call `Engine::run`** (it `process::exit`s on bad config). The
cancellation-token `Stream::run` and its `tokio::select!` abort exist on **git HEAD only** — pin
the rev, re-verify the signature on every bump. The registry/plugin API is not semver-stable.

### R4 — Query safety is the control plane's job, server-side, never the caller's.
For SQL datasources the **user query is the input query** (pushed down so `WHERE`/`LIMIT` run in
the DB; DataFusion pipeline is reserved for non-SQL/cross-source shaping — confirm pushdown in
M0). Every query runs under: a **read-only DB role** (no DDL/DML), a server-enforced **statement
timeout**, **max-rows / max-bytes caps + forced `LIMIT`**, and **cancellation** wired to client
disconnect. On a datasource DB **shared across tenants**, inject a per-tenant predicate — the
data side has no RLS.

### R5 — Tenancy = RLS, bound correctly, proven by test.
nexus-api connects under a **non-`BYPASSRLS`, non-owner** runtime role; tenant tables set `FORCE
ROW LEVEL SECURITY`. A shared middleware opens a transaction and `SET LOCAL app.tenant_id` from
`Principal.tenant_id`; **every** tenant-scoped query runs inside it (a bare pooled query leaks
the previous tenant's GUC). **Grants and internal refs key on immutable ids, not slugs.** A
cross-tenant **pool-reuse leak test** is part of the contract, not optional.

### R6 — Secrets are envelope-encrypted with a real key model.
Datasource connection strings: per-secret data key wraps the secret; a master key (env-injected
v1, pluggable KMS later) wraps the data keys; only ciphertext + key-version live in the DB.
Decrypt **only** inside the runners at stream-build time; `GET /datasources/:id` returns redacted
config (never the password); **every decrypt is audited**.

### R7 — Live is single-node for v1; the stream registry key is the full spec.
The SSE fan-out uses an **in-process** `tokio::broadcast` → live fan-out is **single-`nexus-api`-
node for v1** (scaling out later needs a shared bus; state it, don't discover it). Reuse a
running stream only when **(stream spec + datasource + tenant + required permission)** all match
— keying on "source" alone leaks across tenants. Define heartbeat, slow-subscriber (`Lagged`)
behavior, `Last-Event-ID` resume, and teardown (refcount → cancel on last unsubscribe).

### R8 — SSE auth is not Bearer.
Native `EventSource` cannot send an `Authorization` header. The live route authenticates via a
**short-lived signed stream token** (minted by a REST call, passed in the URL) **or** an
`HttpOnly` cookie (+ CSRF/CORS). REST stays Bearer. **Open decision — pick one before M0.5.**

### R9 — The Collector is bounded.
It buffers `RecordBatch`es in-process, so the sink enforces row/byte/wall-clock caps and aborts
on breach. Large results **page (cursor)** or **stream the HTTP response** — never
buffer-then-serialize an unbounded `SELECT *`.

### R10 — Contracts in `nexus-spi`; handlers ≤20 lines.
REST DTOs (decorated `utoipa::ToSchema`) live in `nexus-spi`; **OpenAPI is the single source of
truth** for the frontend client. A route handler does four things — extract input, call a
runner/store function, shape the DTO, return — and changes only if you swap transports.

### R11 — Test-driven: the test comes first.
**Red → green → refactor**, per [../README.md](../README.md) §5. Write the failing test before
the impl. Unit tests inline (`#[cfg(test)] mod tests`) for pure functions; integration tests in
`tests/` **mirroring `src/` one-to-one** (`src/runner/query.rs` → `tests/runner/query_test.rs`).
DB tests use **real Postgres via testcontainers** — never a stubbed store; runners hit real
engines, not canned rows (README §6). The M0 engine-seam test and the R5 pool-reuse test are
written first and checked in.

### R12 — Comments explain *why*; `nexus-spi` is add-only within a major.
Doc-comments on public items (purpose, defaults, edge cases). No `// FIXED:` / status-banner
comments; TODOs carry a name. Contract surfaces are add-only within a major; breaking changes
bump the crate + client + binary together.

## Repo layout

```
nexus/backend/                         <- cargo workspace
  Cargo.toml                           <- workspace + pinned arkflow git rev
  crates/
    nexus-spi/         <- R2/R10. Contracts only. REST DTOs (utoipa), errors, ids.
      src/
        lib.rs
        error.rs
        id.rs                          <- re-export starter-spi Id<T> / newtypes
        dto/
          datasource/  {list,get,create,update,test,query,shared}.rs
          dashboard/   {list,get,create,update,delete,star,shared}.rs
          panel/       {list,create,update,delete,shared}.rs
          stream/      {create,event,shared}.rs
          alert/       {rule,event,shared}.rs
          flow/        {list,create,update,shared}.rs
          me.rs                        <- GET /api/v1/me response (the /auth/me gap)

    nexus-engine/      <- R3/R4/R7/R9. THE KEYSTONE: ArkFlow bridge.
      src/
        lib.rs
        sink/
          mod.rs                       <- register the two outputs
          collector.rs                 <- Collector output sink (bounded, drains to caller)
          sse.rs                       <- SSE output sink (broadcast)
          cap.rs                       <- row/byte/time caps shared by both sinks
        runner/
          mod.rs
          query.rs                     <- QueryRunner: 1-shot Stream → collector → JSON
          live.rs                      <- LiveRunner: unbounded Stream → sse broadcast
          cancel.rs                    <- CancellationToken plumbing (R3 abort path)
        registry/
          mod.rs
          inputs.rs                    <- register_input_builder(...) for custom inputs
          outputs.rs                   <- register_output_builder("collector"|"sse")
        arrow_json.rs                  <- RecordBatch → JSON
        stream_registry.rs             <- key=(spec+datasource+tenant+perm); refcount
        flow/
          mod.rs
          manager.rs                   <- FlowManager: run saved flows
          start.rs  stop.rs

    nexus-store/       <- R5/R6. sqlx persistence + RLS + migrations.
      src/
        lib.rs
        pool.rs                        <- pool under the runtime (non-BYPASSRLS) role
        tenant_tx.rs                   <- open tx + SET LOCAL app.tenant_id (R5)
        datasource/  {list,get,insert,update,delete}.rs
        datasource/secret.rs           <- envelope encrypt/decrypt boundary (R6)
        dashboard/   {list,get,insert,update,delete,star}.rs
        panel/       {list,insert,update,delete}.rs
        alert/       {rule_*,event_*}.rs
        flow/        {list,insert,update,delete}.rs
      migrations/                      <- *.sql, ordered AFTER starter migrations (exempt R1)

    nexus-api/         <- THE BINARY. Axum server + transport. Handlers ≤20 lines (R10).
      src/
        main.rs                        <- ~100 lines: compose routers, wire engine+store, serve
        state.rs                       <- AppState (pools, registries, engine handles)
        middleware/
          auth.rs                      <- Principal extraction (delegates starter-auth-users)
          tenant.rs                    <- wraps tenant_tx (R5)
          authz.rs                     <- starter-authz grant checks
          stream_token.rs              <- mint/verify signed SSE token (R8)
        routes/
          mod.rs                       <- Router::new().merge(...) wiring only
          me/get.rs
          health/get.rs
          datasources/ {mod,list,get,create,update,delete,test,query}.rs
          dashboards/  {mod,list,get,create,update,delete,star}.rs
          panels/      {mod,list,create,update,delete}.rs
          query/       {mod,run}.rs
          streams/     {mod,create,subscribe,delete}.rs    <- subscribe.rs = SSE
          alerts/      {mod, rules/{list,get,create,update,delete}, events/list}.rs
          flows/       {mod,list,create,update,delete,start,stop}.rs
      tests/                           <- mirrors src/ (R11)

  docs/  -> ../docs/  (this scope lives in nexus/docs/session/backend/)
```

Four crates. Mirror the rubix shape: contracts → logic → binary, verb-per-file throughout.

## Dependency arrow

```
starter-spi
   ↑
nexus-spi
   ↑
   ├── nexus-store     (sqlx; + starter-store-postgres pattern)
   ├── nexus-engine    (arkflow-core + arkflow-plugin, git-pinned)
   │
   └──────────────► nexus-api (binary)
                     + starter-auth-users  (mounts /auth/*)
                     + starter-authz       (mounts /authz/grants)
                     + starter-server      (Axum, SSE, OpenAPI, middleware)
```

Never the reverse. `nexus-spi` depends on nothing internal; nothing forks ArkFlow or
`starter-*`.

## What we use from starter

Nexus lives **inside the starter monorepo** (`/home/user/code/rust/starter`), so these are
in-repo workspace crates under `crates/` — path-deps, not external. ArkFlow is the **only**
external dependency (a pinned git rev; nothing about it is vendored in-tree).

| Capability | Crate | Note |
|---|---|---|
| Contracts (`Id<T>`, `Error`, `Page<T>`, `Principal`, `PolicyEngine`, `SecretStore`) | `crates/starter-spi` | `nexus-spi` re-exports these |
| AuthN + users + teams + tenants | `crates/starter-auth-users` | mount its router at `/auth/*` |
| AuthZ (team/user → resource grants, audit) | `crates/starter-authz` | grant checks + `/authz/grants` |
| HTTP server, SSE, OpenAPI (utoipa), middleware | `crates/starter-server` | `nexus-api` composes routers onto it |
| Postgres building blocks + testcontainers pattern | `crates/starter-store-postgres` | pool, migrations, paging; R5 RLS rides on it |
| Layered config (env > file > default) | `crates/starter-config` | bind addr, DB URLs, key handles |
| Tracing / metrics / middleware | `crates/starter-observability` | nexus-api's own telemetry |
| Secrets (master key for R6 envelope encryption) | `crates/starter-secrets-file` (default), `-keyring` (opt-in) | datasource secret wrapping |
| i18n / user prefs (units, time, theme) | `crates/starter-i18n`, `crates/starter-prefs` | if/when handlers emit user-facing text |
| **Engine core** (Stream, registry, Arrow, DataFusion) | **`arkflow-core` + `arkflow-plugin`** | **external git dep**, pinned rev, **not forked** (R3) |

Maybe-later (present in `crates/`, not v1): `starter-store-warehouse` + `starter-warehouse`
(ClickHouse history), `starter-grpc` (a second transport), `starter-audit`/`starter-changelog`
(write audit beyond R6 decrypt logs), `starter-skills`/`starter-flow-*` (only if Nexus ever grows
an agent surface — out of scope now).

**Deliberately not used:** `starter-auth-token` (single-owner bearer — Nexus is multi-user),
`starter-store-sqlite` (Postgres + RLS only). If a capability is missing from a `starter-*`
crate, fix it **upstream** and consume it — don't grow a parallel crate in nexus (the rubix R2
rule applies here too).

## Where does my code go? — decision tree

1. **Wire type** (REST DTO) → `nexus-spi/src/dto/<noun>/<verb>.rs`; regenerate OpenAPI.
2. **Talks to ArkFlow** (a sink, a runner, the registry, Arrow→JSON, a flow) → `nexus-engine`.
3. **Talks to Postgres** (a query, a migration, RLS, secrets) → `nexus-store`.
4. **HTTP route** → `nexus-api/src/routes/<noun>/<verb>.rs`, ≤20 lines (R10). Domain stays in
   engine/store.
5. **Auth/tenant/authz/SSE-token wiring** → `nexus-api/src/middleware/`.
6. **A new datasource connector** → `register_input_builder(...)` in
   `nexus-engine/src/registry/inputs.rs` (no core change, no fork).
7. **Migration** → `nexus-store/migrations/`, ordered after starter's.
8. **Unsure?** → re-read `../../scope/NEXUS.md` and FILE-LAYOUT.md, then ask. One sentence beats a
   wrong-direction refactor.

## Building in parallel with the frontend (contract-first)

**Yes — backend and frontend run concurrently, gated on one shared artifact: the OpenAPI
document emitted from `nexus-spi`.** That contract is the seam (R10). The order is *contract
first*, not *backend first*:

1. **Land the contract first.** Define DTOs + routes in `nexus-spi` (work-unit **U4**) and emit
   `openapi.json`. This is the single cross-team unblocker — it lets the frontend codegen its
   typed client (`@nube/starter-client-ts`) before a single handler is implemented.
2. **Then both sides build against it.** Backend implements engine/store/handlers behind the
   contract; frontend builds its UI + hooks against the generated client. Neither waits on the
   other's internals.
3. **Frontend integration needs a live backend (no mocks — README §6).** Because the UI never
   fakes data, its *integration* tests and dev runtime point at a **real `nexus-api`** (a dev
   instance or testcontainers). So a backend endpoint must exist before the matching screen is
   *verified* — but the screen's code, hooks, and component tests are written in parallel.
4. **Contract is add-only within a major (R12).** Once `openapi.json` is published for an area,
   changes are additive; a breaking change is a coordinated bump on both sides.

Practically: **dispatch U4 (nexus-spi) before everything**, publish the OpenAPI snapshot, then
fan out backend U1–U10 and the frontend work-units (W*) at the same time.

## Phases (strictly ordered; each assumes the previous landed)

Mirrors [`../../scope/NEXUS.md`](../../scope/NEXUS.md) §9, expanded into entry-gates + exit-criteria so
each is a dispatchable unit.

### Open decision before M0 (Risk #17) — does ArkFlow sit on the M0 critical path?
ArkFlow's value is *streaming connectors* (M3); M0–M2 (Postgres query + dashboards) could run on
**DataFusion + sqlx directly** and bring ArkFlow in at M3. Resolve before writing M0: either
(a) prove the ArkFlow seam now, or (b) DIY the query path and defer the engine dependency. Phases
below assume (a); if (b), M0/M0.5 retarget the runner onto DataFusion+sqlx and ArkFlow moves to
an M3 entry-gate.

### M0 — Prove the engine seam (make-or-break)
- `nexus-engine` cargo-git-dep on arkflow-core/plugin (pinned rev). Build the **Collector sink**
  (bounded, R9) + `QueryRunner`.
- `POST /query` → one-shot `Stream{ input: sql→Timescale, pipeline:[], output: collector }` →
  Arrow→JSON. Hit `samples` / `demo_bi`.
- **Exit:** real rows returned; the bounded-collector + clean finite-stream-termination test
  (R11) is green; pushdown confirmed (R4).

### M0.5 — SSE seam
- Build the **SSE sink** + `LiveRunner` + `stream_registry` (R7). Mint/verify the **signed
  stream token** (R8 — resolve the auth mechanism here).
- `GET /streams/:id` streams a `generate`/MQTT input to a browser.
- **Exit:** live values tick in a browser without a Bearer header; last-subscriber teardown works.

### M1 — Identity & multi-tenancy
- Mount `starter-auth-users` + `starter-authz`; add `GET /api/v1/me`. Wire the `tenant_tx`
  middleware (R5) and the read-only datasource role + query guards (R4).
- Datasource CRUD (tenant-scoped) + secret envelope encryption (R6).
- **Exit:** tenant-isolated query end-to-end under the runtime role; the **cross-tenant
  pool-reuse leak test passes**; DDL is rejected by the read-only role.

### M2 — Dashboards
- Port the dashboard/panel schema + REST from strata's proven contract; grants/refs on immutable
  ids. 3–4 panel query shapes.
- **Exit:** a dashboard's panels each run their query (R4) tenant-isolated and authorized.

### M3 — Live + breadth + alerting
- Live panels in the canvas; more connectors (Kafka/MQTT/Modbus) via `register_input_builder`;
  the `FlowManager` runs a saved weather flow; templates.
- **Alerting sub-design** (its own doc): evaluator/scheduler, alert **state machine**
  (pending/firing/resolved), dedup/silences, notification channels. Don't ship `/alerts/*` as
  bare CRUD.
- **Exit:** a saved flow ingests on a schedule; a live panel streams; one alert rule fires end to
  end through a channel.

### M4 — Extensibility
- Backend datasource plugins via the ArkFlow registry (incl. a DataFusion file/federation source
  — its own credentials/schema/governance work).
- **Exit:** a new connector ships as a registered input with no core change and no fork.

## Smoke tests (before merging anything)

- **"Engine seam returns rows" (M0):** `POST /query` returns real rows; the finite Stream
  cleanly terminates and the bounded collector drains.
- **"RLS can't leak across a pooled connection" (R5):** two tenants served back-to-back on one
  pooled connection cannot read each other's rows. Checked-in test.
- **"Read-only role blocks writes" (R4):** a `DROP`/`INSERT` through the query path is rejected by
  the datasource role, not by string-matching.
- **"SSE without Bearer" (R8):** a browser `EventSource` subscribes and authenticates via the
  signed token / cookie path; no `Authorization` header involved.
- **"No ArkFlow fork" (R3):** `cargo tree` shows arkflow as a git dep; grep finds no
  re-implemented `Engine`/registry. `Engine::run` is never called.
- **"Swap REST for gRPC" (R10):** pick a handler; only route wiring + DTO shaping would change.
- **"AI loads context cleanly" (R1):** every `*.rs` under 400 lines, name = one concept, test at
  the mirrored path.
- **"Secrets never leave" (R6):** `GET /datasources/:id` response contains no plaintext secret;
  every decrypt emits an audit row.

## Subagent work-units (parallelizable)

The verb-per-file layout exists so these run independently. Suggested first-wave units (each =
one folder/file set, one owner, no cross-conflicts):

| Unit | Files | Depends on |
|---|---|---|
| **U1 Collector sink** | `nexus-engine/src/sink/{collector,cap}.rs`, `registry/outputs.rs` | arkflow pin |
| **U2 QueryRunner + Arrow→JSON** | `nexus-engine/src/runner/{query,cancel}.rs`, `arrow_json.rs` | U1 |
| **U3 SSE sink + LiveRunner** | `nexus-engine/src/sink/sse.rs`, `runner/live.rs`, `stream_registry.rs` | U1 |
| **U4 nexus-spi DTOs** | `nexus-spi/src/dto/**`, `error.rs`, `id.rs` | — |
| **U5 store: datasources + secret** | `nexus-store/src/datasource/**`, `tenant_tx.rs`, `pool.rs` | U4 |
| **U6 store: dashboards/panels** | `nexus-store/src/{dashboard,panel}/**`, `migrations/` | U4 |
| **U7 routes: query + streams** | `nexus-api/src/routes/{query,streams}/**`, `middleware/stream_token.rs` | U2,U3,U4 |
| **U8 routes: datasources/dashboards/panels** | `nexus-api/src/routes/{datasources,dashboards,panels}/**` | U4,U5,U6 |
| **U9 middleware: auth/tenant/authz** | `nexus-api/src/middleware/{auth,tenant,authz}.rs`, `state.rs`, `routes/me/get.rs` | U4,U5 |
| **U10 binary wiring** | `nexus-api/src/main.rs`, `routes/mod.rs`, `routes/health/get.rs` | all |

Each unit ships its mirrored test (R11). U1+U4 are the unblockers — dispatch them first.

## Non-goals (v1)

- **No horizontal multi-node live.** In-process broadcast; single nexus-api node (R7). Shared-bus
  scale-out is later.
- **No heavy ETL/CDC/lake replication.** Saved *flows* (light ingestion) are in; batch ETL is out
  (NEXUS.md §13).
- **No native PromQL/LogQL.** Only sources ArkFlow already speaks; real PromQL/LogQL is its own
  product-sized effort.
- **No second identity/authz model.** `starter-auth-users` + `starter-authz` only; no Casbin/
  OpenFGA unless relationship depth demands it later.
- **No SQLite in production.** Postgres + RLS only (SQLite is dev/single-node, app-enforced
  tenancy).
- **No ArkFlow fork.** Pinned git dep; missing capability → upstream, not fork.
- **No frontend code here.** Lives in `nexus/ui/` ([`../ui/SCOPE.md`](../ui/SCOPE.md)).

## Bottom line

**ArkFlow is the engine; `nexus-api` is the product.** Four crates (contracts → engine → store →
binary), verb-per-file, ≤400 lines. The keystone is the two-sink/two-runner seam (M0/M0.5); the
non-negotiables are query safety (R4), RLS-bound tenancy (R5), bounded collector (R9), and
not-Bearer SSE auth (R8). Everything else is CRUD over Postgres behind `starter-authz`.
