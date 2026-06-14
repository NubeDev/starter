# Nexus — Architecture & Stack

> **Status (2026-06-09):** **Architecture locked; implementation contracts pending M0/M1
> verification.** The *direction* (ArkFlow core + nexus-api control plane + starter-auth/
> authz + nexus-ui) is settled. Several wire-level contracts below (ArkFlow cancellation,
> auth endpoint shapes, `/auth/me` payload, the nexus-ui migration) are **not yet proven in
> code** and are flagged inline — do not treat them as final until M0/M1 close them.
> Supersedes `strata/STACK.md` (strata is now reference-only — see §10).
>
> **Safety/operational contracts added after peer review (2026-06-09)** and still pending
> proof: **SSE auth** (`EventSource` can't carry a Bearer token — §5.3/§7), **SQL execution
> governance** (read-only role, caps, two-layer-SQL resolution — §5.2), **RLS operational
> mechanics** (§4), **datasource secret/key management** (§4), **extension code-execution
> security** (§7), and **stream lifecycle + single-node scale boundary** (§5.3).

A self-hosted, open-source **observability / BI dashboard platform** — a Grafana
alternative with full UI/UX control — built on the **ArkFlow** stream engine as its core,
with a Rust control plane providing auth/teams/users, REST, and SSE.

- **Backend core:** ArkFlow (Rust, Apache-2.0) — connectors + DataFusion SQL + plugin registry.
- **Control plane:** `nexus-api` (new Axum crate) — auth, users, teams, REST, SSE.
- **AuthN:** `starter-auth-users`. **AuthZ:** `starter-authz` (in-repo). 
- **Frontend:** `nexus-ui` (base: [`nexus-ui/OVERVIEW.md`](./nexus-ui/OVERVIEW.md)) — React
  **19** + React Router + shadcn/ui + TanStack Query + zustand; host for the in-repo
  runtime-federation extension SDK.

---

## 1. What We're Building

A platform where users connect **any data source**, run **ad-hoc queries**, build
**drag-and-drop dashboards**, and stream **live realtime panels** — with **teams** and
**per-page/per-dashboard permissions**. Extensible by plugins on both ends: a *data
source* is an ArkFlow input plugin; a *panel type* is a frontend module.

Requirements driving this doc: **auth / teams / users**, **SSE**, **REST**, and a
**plugin system** for extending the backend and data sources.

---

## 2. Why ArkFlow Is the Core

ArkFlow (github.com/arkflow-rs/arkflow, Apache-2.0) gives us, for free, the layer that is
otherwise the hardest part of a dashboard backend:

| Capability | ArkFlow provides |
|---|---|
| **Data-source connectors** | Kafka, MQTT, **Modbus**, HTTP, Redis, NATS, Pulsar, WebSocket, SQL (MySQL/PG/SQLite), File (CSV/JSON/Parquet/Arrow on S3/GCS/Azure/HDFS) |
| **Query engine** | DataFusion SQL over Arrow `RecordBatch` (joins, windows, aggregates, UDFs) |
| **Plugin system** | `register_*_builder()` registry — `pub`, lookup-by-name, our exact "extend data sources" mechanism |
| **Streaming** | continuous Arrow batches → natural fit for **SSE live panels** |
| **Transforms** | SQL, VRL, Python UDF, JSON, Protobuf |

What ArkFlow **does not** provide (we build it — that's `nexus-api`):
users, auth, teams, persistence, a product REST API, SSE — its only HTTP is a health check.

**Decision: depend on ArkFlow, do not fork.** Its registry (`register_*_builder`) and
`Stream` / `StreamConfig::build` are public. We pull arkflow-core + arkflow-plugin as a
cargo `git =` dependency and add our server crate + custom plugins in our own repo. We
drive `Stream` directly and **avoid `Engine::run`** (it calls `process::exit` on bad
config — unsafe to embed).

> ⚠️ **Pin a git rev, not the crates.io release.** `Stream::run(&mut self,
> cancellation_token)` and its `tokio::select!`-based abort (§5.3) exist in **git HEAD** but
> **not in the published docs.rs release** (which exposes `run(&mut self)` with no
> cancellation). Our SSE/abort design *requires* the git version. Pin the exact rev and
> re-verify on every bump.

**Why ArkFlow over other Rust stream engines.** We want an *embeddable library with a public
plugin registry* and broad connectors (incl. **Modbus**). The alternatives don't fit that
shape: **Arroyo** is a heavier *distributed platform* (own control plane/UI) — a fallback if
ArkFlow's embeddability fails M0, not an embeddable core; **Vector** is logs/metrics routing,
not general SQL analytics; **Tremor** is event shaping with its own language; **RisingWave**
is a streaming-SQL *database* (different paradigm — a candidate future *data source*/serving
layer, not the embedded core); **SeaStreamer** is a toolkit (build-it-yourself); **Timely/
Differential Dataflow** are low-level dataflow libs (too much engineering). ArkFlow remains
the best fit *for embedding*; RisingWave (serving layer) and Arroyo (M0 fallback) are kept in
our back pocket.

---

## 3. Architecture

```
┌─ nexus-ui  (React + React Router + shadcn/ui + TanStack Query) ───────────┐
│   dashboards · panel canvas · query editor · LIVE panels (SSE) · admin    │
└────────────────────────────┬──────────────────────────────────────────────┘
              REST /api/v1/*   +   SSE /api/v1/streams/:id
┌────────────────────────────▼──────────────────────────────────────────────┐
│  nexus-api   (NEW Axum crate — the control plane)                          │
│                                                                            │
│   HTTP layer (Axum):                                                       │
│     • REST: auth, users, teams, datasources, dashboards, panels, alerts    │
│     • SSE:  /streams/:id  (live panels)                                    │
│   Identity:                                                                │
│     • AuthN  → starter-auth-users  (Principal: user, tenant, memberships)  │
│     • AuthZ  → starter-authz       (grants: team/user → dashboard/page)    │
│   Persistence:                                                             │
│     • sqlx → Postgres/Timescale (dashboards/panels/alerts/datasource cfg)  │
│     • Multi-tenant RLS bound to Principal.tenant_id                        │
│   Engine integration:                                                      │
│     • QueryRunner → 1-shot ArkFlow Stream → COLLECTOR sink → Arrow→JSON    │
│     • LiveRunner  → unbounded ArkFlow Stream → SSE sink → broadcast        │
│     • PluginRegistry → upstream ArkFlow plugins + OUR custom in/out plugins│
└────────────────────────────┬──────────────────────────────────────────────┘
          cargo git dependency (not a fork)
┌────────────────────────────▼──────────────────────────────────────────────┐
│  arkflow-core + arkflow-plugin   (unchanged upstream, Apache-2.0)          │
│   Engine/Stream/registry/Arrow · connectors · DataFusion · transforms      │
└────────────────────────────────────────────────────────────────────────────┘
                  data sources: Timescale · Prometheus* · Kafka · MQTT · Modbus · files
```

\* Prometheus/Loki: ArkFlow has no native PromQL/LogQL input. A custom HTTP-proxy input that
forwards a raw query string is small; **real PromQL/LogQL compatibility is product-sized**, not
a "small plugin." Treat "any data source" as *any source ArkFlow already speaks* (SQL/Kafka/
MQTT/Modbus/HTTP/files) — first-class PromQL/LogQL is its own scoped effort, not a v1 freebie.

---

## 4. Identity: AuthN, Users, Teams, AuthZ

### AuthN + users + teams → `starter-auth-users`
The in-repo crate owns identity: `Principal` (`subject`, `role`, `scopes`, `tenant_id` —
`starter-spi/src/auth/principal.rs`), users, tenants, memberships, teams. `nexus-api`
authenticates each request → `Principal`, injected into handlers. Replaces ArkFlow's
(nonexistent) auth and strata's Nucleus.

**nexus-api mounts the existing crate routers — it does not invent its own auth API.** The
real, verified surface:
- `starter-auth-users`: `POST /auth/login`, `POST /auth/token`, `POST /auth/logout`,
  `GET /auth/me`, `POST /auth/signup`. **There is no `/auth/refresh`** — bearer tokens come
  from `POST /auth/token`; session renewal is re-`login`.
- Teams are **tenant-scoped**: `…/v1/tenants/{id}/teams`, `…/v1/tenants/{id}/members`
  (`starter-auth-users/src/routes/tenants.rs`).
- Grants: `…/v1/authz/grants` (`starter-authz`).

> ⚠️ **`/auth/me` is insufficient for the frontend as-is.** `MeResponse` returns only
> `{subject, email, role}` — no `tenant_id`, `scopes`, or teams — yet the UI needs them for
> `usePrincipal()` / `useCan()`. **Action:** nexus-api exposes a richer context endpoint
> (e.g. `GET /api/v1/me` returning `tenant_id`, teams, and effective permissions) rather than
> relying on `starter-auth-users`'s `/auth/me`.

### AuthZ → `starter-authz` (in-repo) — **chosen over an external service**
The requirement "assign users/teams to a page" is relationship-based authorization.
We use the in-repo **`starter-authz`** policy engine (grants, ACL summaries, conditions,
tenant scoping, resource-instance providers).

**Why starter-authz, not an external project:**
- It already exists in this repo and pairs with `starter-auth-users` — no second identity/
  authz model to reconcile (the reason **Casbin was rejected**).
- Rust-native, in-process — **no extra service** to run/operate.
- Directly models team/user → resource grants, which is exactly page/dashboard permissions.

**The one open-source alternative worth naming — OpenFGA** (CNCF, Apache-2.0,
Google-Zanzibar): a relationship-graph authz *service*. Reach for it **only if** permission
relationships get deep enough that a dedicated Zanzibar store earns its operational cost
(nested folders, sharing graphs, cross-tenant delegation). Until then, starter-authz wins
on simplicity. (SpiceDB = same category, AGPL core — prefer OpenFGA's Apache-2.0 if we ever
externalize.)

**Enforcement points:**
- Persisted resources (dashboards/panels) — `starter-authz` grant check in handlers +
  Postgres RLS for defense in depth.
- Query/live execution — a `Principal` may only run a `Stream` against a datasource config
  its tenant owns; enforced in `QueryRunner`/`LiveRunner` before building the stream.

### Grant on immutable IDs, not slugs
Dashboards expose a human `slug` for routes and links, but **grants and all internal APIs key
on an immutable `dashboard_id` (UUID)**. A slug is a mutable display alias — if a grant or a
panel reference pointed at a slug, renaming a dashboard would silently orphan its permissions
and break shared links. Resolve `slug → id` at the request edge; everything below the handler
(authz checks, RLS, panel refs, audit) uses the id.

### RLS operational mechanics — the part that actually enforces tenancy
Choosing RLS is not enough; the mechanics are the contract (verified in M1):
- nexus-api connects under a **runtime role without `BYPASSRLS`**, and **not the table owner**
  (owners bypass RLS). Add `FORCE ROW LEVEL SECURITY` so even an owner path stays covered.
  Isolation is proven under *this* role — never superuser.
- Tenant context is bound **per transaction**: `SET LOCAL app.tenant_id = $1`; policies read
  `current_setting('app.tenant_id')`. `SET LOCAL` is transaction-scoped, so it can't leak into a
  pooled connection's next checkout — **but only if every query runs inside a transaction that
  sets it first.** A bare pooled query with no surrounding `SET LOCAL` sees the *previous*
  tenant's GUC. A shared middleware opens the tx and sets the GUC from `Principal.tenant_id`;
  all three crates' queries (auth-users, authz, nexus-api) run inside it.
- **Tests are part of the contract:** prove that two tenants served back-to-back on the same
  pooled connection cannot read across tenants.

### Datasource secrets & key management
"Encrypted config" is named in the topology but needs a model before storing real credentials:
- **Envelope encryption** — a per-secret data key wraps the connection string; data keys are
  wrapped by a master key held *outside* the DB (env-injected for v1, pluggable KMS later).
  Only ciphertext (+ key version) lives in `datasources`.
- **Rotation** — master-key rotation re-wraps data keys without re-encrypting every secret.
- **Decrypt boundary** — secrets decrypt only inside `QueryRunner`/`LiveRunner` at stream-build
  time; never returned over the API. `GET /datasources/:id` returns redacted config
  (host/db/user, never the password).
- **Audit** — log every decrypt (who / when / which datasource).

---

## 5. Engine Integration — the part we build

ArkFlow is streaming-first and has **no "return rows to caller" output**. The bridge to a
request/response + SSE product is **one custom Output plugin with two modes**:

### 5.1 Custom sink plugin (the keystone)
- **Collector sink** — accumulates `RecordBatch`es in-process; on stream completion the
  `QueryRunner` drains them and serializes Arrow → JSON. Powers **request/response panel
  queries**.
- **SSE sink** — forwards each `RecordBatch` to a `tokio::sync::broadcast` channel; SSE
  subscribers receive JSON chunks. Powers **live panels**.

Both register via the public `register_output_builder("collector" | "sse", …)`.

> **The collector must be bounded.** It buffers `RecordBatch`es in-process, so the sink
> enforces hard **row / byte / wall-clock caps** and aborts the stream when exceeded — an
> unbounded `SELECT *` would OOM nexus-api. Large result sets should **page (cursor)** or
> **stream the HTTP response** rather than buffer-then-serialize. The cap lives in the sink, not
> in the (untrusted) query.

### 5.2 QueryRunner (`POST /query`, `POST /datasources/:id/query`)
1. Authn/authz + tenant-scope the datasource.
2. Build `StreamConfig { input: <datasource>, pipeline: [sql], output: collector }`.
3. `StreamConfig::build()` → `Stream::run(cancellation_token)`.
4. Drain collector → Arrow→JSON → HTTP response.

**Two-layer SQL — resolve the layer explicitly (proposed; M0 must prove).** ArkFlow runs an
*input* query (pushed to the source) **and** a *pipeline* SQL (DataFusion, in-memory over Arrow).
The docs must say which one the user's SQL is, because both naive readings break:
- user SQL as the **pipeline** → the input first pulls the whole table into memory, then filters
  → no predicate pushdown, unusable on real tables;
- user SQL as the **input** but with a permissive role → arbitrary SQL hits the source directly.

**Resolution:** for SQL datasources the **user query is the input query** (pushed down so
`WHERE`/`LIMIT` run in the database), executed under the guards below; the DataFusion pipeline is
reserved for shaping non-SQL inputs (Modbus/MQTT/HTTP) and cross-source transforms. M0 confirms
pushdown actually happens.

**Query safety is mandatory and server-side — not the caller's job (Risk #2/#10/#11):**
- Datasource connections use a **read-only DB role**; no DDL/DML can reach a source.
- **Server-enforced** statement timeout, max-rows and max-bytes caps, and a forced `LIMIT` —
  never trusted from the request body.
- **Cancellation** wired to the `CancellationToken` *and* to client disconnect.
- For a datasource DB **shared across tenants**, inject a per-tenant predicate or use a
  tenant-bound DB role — datasource *ownership* alone does not isolate rows *inside* a shared DB
  (the data-side has no RLS; only the metadata DB does).
- Optional per-user/team/tenant **concurrency + resource quotas** to bound one dashboard's
  fan-out of N panel queries.

### 5.3 LiveRunner + SSE (`GET /api/v1/streams/:id`)
1. Client requests a live panel; authz + tenant check.
2. Build `StreamConfig { input: <kafka|mqtt|modbus|…>, pipeline: [sql], output: sse }`.
3. Spawn `Stream::run(token)`; SSE handler subscribes to the broadcast and emits
   `data: <json>` events.
4. On client disconnect → cancel the `CancellationToken` → stream stops. **Concrete abort
   path (git HEAD):** `Stream::do_input` runs `tokio::select! { _ = token.cancelled() => break,
   r = input.read() => … }`, so cancelling the token interrupts an in-flight `read()` and
   breaks the loop promptly. (Requires the pinned git rev — §2.) One stream per live source,
   fanned out to N subscribers; refcount subscribers and cancel when the last disconnects.

**Registry key — not "source" alone.** Reuse a running stream only when the full spec matches:
**(stream spec + datasource + tenant + required permission)**. Two tenants querying the "same"
Kafka topic must **not** share a broadcast — keying on source alone leaks data across tenants.

**Lifecycle contract (define before M0.5):** heartbeat/keepalive interval; slow-subscriber
behavior (`tokio::broadcast` drops as `Lagged` — decide: disconnect the laggard vs widen the
buffer, and how the client learns it missed events); reconnect + `Last-Event-ID` resume
semantics; per-event IDs; registry ownership and teardown (refcount → cancel on last
unsubscribe, and what happens to live panels on a deploy/restart — today they all die silently).

**SSE auth — `EventSource` cannot send a Bearer header.** The native browser `EventSource` API
has no header support, so the REST Bearer model does **not** carry over to the live route.
Authenticate it via **one** of: a secure `HttpOnly` cookie (+ CSRF/CORS rules), a **short-lived
signed stream token in the URL**, or a fetch-based SSE reader (e.g. `@microsoft/fetch-event-
source`) that can set headers. Pick one in M0.5.

**Scale boundary.** The broadcast is **in-process**, so live fan-out is **single-`nexus-api`-
node for v1**: a stream on replica A can't serve a subscriber that lands on replica B. Scaling
out later needs a shared bus (NATS/Redis). State this as a v1 constraint rather than discovering
it at deploy time — and note it sits in tension with "needs multi-user concurrent writes" as the
Postgres justification (NEXUS_TOPOLOGY §storage).

### 5.4 Data source = ArkFlow input plugin
A datasource in Nexus = (ArkFlow input `type` + saved connection config + tenant owner).
**Adding a connector = implement `Input` + `InputBuilder`, `register_input_builder(...)`**
in `nexus-api`'s startup. That is the "extend the backend & data sources" capability,
delivered by ArkFlow's registry. No core changes, no fork.

---

## 6. REST + SSE API Surface (v1)

| Area | Endpoints |
|---|---|
| **Auth** (mounted from `starter-auth-users`) | `POST /auth/login`, `POST /auth/token`, `POST /auth/logout`, `GET /auth/me`, `POST /auth/signup` |
| **Frontend context** (nexus-api, NEW) | `GET /api/v1/me` → `{subject, email, role, tenant_id, teams, permissions}` (fills the `/auth/me` gap) |
| **Users/Teams** (tenant-scoped, from crates) | `…/v1/tenants/{id}/teams`, `…/v1/tenants/{id}/members` |
| **Grants** (from `starter-authz`) | `…/v1/authz/grants` |
| **Datasources** | `GET/POST /datasources`, `GET/PUT/DELETE /datasources/:id`, `POST /datasources/:id/test` |
| **Query** | `POST /query`, `POST /datasources/:id/query` (one-shot → JSON) |
| **Live (SSE)** | `POST /streams` (create), `GET /streams/:id` (SSE subscribe), `DELETE /streams/:id` |
| **Dashboards** | `GET/POST /dashboards`, `GET/PUT/DELETE /dashboards/:slug`, `POST /dashboards/:slug/star` (slug is a route alias; **grants/panel-refs key on the immutable id** — §4) |
| **Panels** | `GET/POST /dashboards/:slug/panels`, `PUT/DELETE /panels/:id` |
| **Alerts** | `GET/POST /alerts/rules`, `GET/PUT/DELETE /alerts/rules/:id`, `GET /alerts/events` |
| **Templates** | `GET /templates`, `POST /templates/:slug/use` |
| **Health** | `GET /health` |

> **Prefix:** every product endpoint above is mounted under **`/api/v1`** (the table omits the
> prefix for brevity — `/datasources` ⇒ `/api/v1/datasources`). The `starter-auth-users` routes
> keep their own mount (`/auth/*`); the diagrams' `/api/v1/*` refers to the product surface.

nexus-api **composes** the auth/teams/grants routers from the in-repo crates (verified paths
above) and adds the product endpoints (datasources/query/live/dashboards/panels/alerts). The
dashboard REST shapes are ported/adapted from strata's proven contract — see §10.

> **Alerting is a subsystem, not just these endpoints.** The `/alerts/*` rows are a CRUD
> surface; the *evaluator* is unspecified and needs its own design — rule scheduler/cadence,
> evaluate-over-query-vs-stream, an alert **state machine** (pending/firing/resolved), dedup +
> silences, and **notification channels** (email/Slack/webhook/PagerDuty). Tracked as a Risk
> (§11) and an M3 sub-design, not implied as "three endpoints."

---

## 7. Frontend — `nexus-ui`

> **The UI base is [`nexus-ui/OVERVIEW.md`](./nexus-ui/OVERVIEW.md)** — the source of truth
> for the frontend (app architecture, widget library, design system, the federation contract,
> and the OpenUI roadmap). It is a working **dark-mode IoT/observability dashboard-builder
> mockup**; the running stack is mid-migration to the target below.

**Target stack (dictated partly by the Module Federation contract — see OVERVIEW §7):**

- **React 19** — *singleton-pinned*: must match the rubix extension remotes' shared-React
  major (currently being bumped from 18 → 19).
- **React Router** (host-only) + **shadcn/ui** + **react-hook-form** + **zod**.
- **TanStack Query** — **not optional**: it's one of the host↔extension **shared singletons**
  (one `QueryClient`/cache). This is the data layer over `nexus-api` REST.
- **zustand** — the companion **shared singleton** for client state.
- **No Refine** — it brings its own query runtime extensions can't share, *and* it's
  unnecessary for ~4 CRUD screens. Data layer = plain `fetch` + TanStack Query + context.
- **Panel engine (custom, the multi-week core):** `react-grid-layout` canvas (⚠️ maintenance
  mode — 30-min dnd-kit spike first), **ECharts** for panels (mock currently uses Recharts →
  swap; add uPlot for dense series / AG Grid for tables as needed).
- **Live panels:** subscribe to `GET /api/v1/streams/:id`. **Native `EventSource` can't send a
  Bearer header** — auth via `HttpOnly` cookie or a signed stream-token URL, or use a fetch-based
  SSE reader (§5.3).
- Auth/authz are ordinary `fetch` + context (`usePrincipal()`, `useCan()`), not framework
  providers.

> **Migration debt (tracked in OVERVIEW §6):** bump React 18→19 *(in progress)*, remove
> Refine, add TanStack Query + zustand, swap Recharts→ECharts, point the data layer at
> `nexus-api`. The dashboard/widget **data model** (`nexus-ui/src/data/types.ts`) is
> stack-agnostic and survives the migration — only the provider/chart layers change.

### Extension model — custom SDK federation, **not** Rspack/Webpack MF

`nexus-ui` must be the **host** for the existing rubix UI-extension system, and those
extensions (`com.nubeio.ce`) must keep working unchanged. This is a **custom Vite
library-mode SDK federation** via **`@nube/starter-ext-ui`** (host) + `@nube/starter-ext-sdk-ts`
(remotes) — each extension ships an ESM `remoteEntry.js`, React is shared via the host
importmap, and components are contributed to **named slots** (not routes). **Do not introduce
`@module-federation/*`.** Shared singletons the host registers: `react`/`react-dom`,
`@tanstack/react-query`, `zustand`, and ui-core i18n/preferences. A **Nexus panel type can
ship as an extension** built with this recipe. Full contract in OVERVIEW §7.

> **Loading a `remoteEntry.js` = running trusted code in the user's session.** Federation is
> defined as *integration*; it also needs a *security* model before third-party remotes load:
> a **manifest allowlist**, **checksum-pin or signature** on each `remoteEntry.js`, a **CSP**,
> a version/compat policy, and an explicit **capability boundary on `StarterClient`** (what an
> extension can call / read). For v1 the only remote is in-repo (`com.nubeio.ce`), so this can
> trail — but it must precede loading *any* out-of-repo extension. See OVERVIEW §7.

---

## 8. Rejected Options (and why)

| Rejected | Reason |
|---|---|
| **Cube** | Semantic layer (pre-modeled metrics); Grafana-style is ad-hoc per-panel query. Wrong layer + Node, not Rust. |
| **DataFusion as the foundation** | Can't run PromQL/LogQL; it's a SQL/Arrow engine. We still *get* DataFusion — it's ArkFlow's transform engine. |
| **Refine** | Dashboard product, not admin CRUD; the `nexus-ui` mock already bypasses it. |
| **Casbin** | Would duplicate the in-repo `starter-authz` model. |
| **strata as the base** | Its value (datasource proxy + query) is superseded by ArkFlow; remaining CRUD is tangled with Nucleus. Salvaged as reference (§10). |

---

## 9. Roadmap

**M0 — Prove the engine seam (make-or-break, ~1 day)**
- New `nexus-api` crate → cargo-git-dep on arkflow-core/plugin.
- Build the **Collector sink** plugin.
- One-shot `Stream { input: sql→Timescale, pipeline:[sql], output: collector }` →
  Arrow→JSON over a single `POST /query`. Hit `samples` / `demo_bi`.
- **Acceptance:** real rows returned. Validates request/response-over-streaming.

**M0.5 — SSE seam**
- Build the **SSE sink**; `GET /streams/:id` streams a `generate`/MQTT input to the browser.
- **Acceptance:** live values tick in a browser `EventSource`.

**M1 — Identity & multi-tenancy**
- Wire `starter-auth-users` (Principal) + `starter-authz` grants.
- Port strata's RLS pattern; **verify isolation under the runtime DB role, not superuser**.
- Datasource CRUD, tenant-scoped; QueryRunner authz gate.
- **Acceptance:** tenant-isolated query end-to-end, authorized via starter-authz.

**M2 — Dashboards (+ federation *host*)**
- Port dashboard/panel schema + REST; react-grid-layout canvas; 3–4 ECharts panel types;
  query editor + Explore.
- **Stand up the `@nube/starter-ext-ui` host shell + `<ExtensionSlot>`s and mount the existing
  `com.nubeio.ce` remote unchanged.** Rubix compatibility is a *hard requirement*
  (OVERVIEW §7), so the host runtime cannot wait for M4 — building nexus-ui on the host
  provider from the start is what makes the React-19/TanStack/zustand singleton locks pay off.

**M3 — Live + breadth**
- Live panels (SSE) in the canvas; more connectors (Kafka/MQTT/Modbus); templates.
- **Alerting (its own sub-design, not just CRUD):** evaluator/scheduler, alert state machine,
  notification channels — see the §6 note and Risk §11.

**M4 — Extensibility breadth ("one day")**
- Extension **ecosystem** beyond the in-repo remote: 3rd-party remotes (gated behind the
  extension *security* model — allowlist/signing/CSP/capabilities, §7), panel-type-as-extension
  breadth, and the marketplace/loading story. The *host runtime* itself lands in M2; M4 is about
  loading remotes we don't own.
- Backend data-source plugins via the ArkFlow registry (incl. a DataFusion file/federation
  source — with its own credentials/schema/governance work).

---

## 10. strata — Reference Only (salvage, don't run)

`strata/` stays on disk as a **design reference**, not a base.

| Salvage (port deliberately) | Leave behind |
|---|---|
| DB schema: dashboards/panels/alerts/templates | Nucleus auth + tenant-derivation coupling |
| 29-endpoint REST contract (`strata/SITEMAP.md`) | Datasource proxy modules (ArkFlow replaces) |
| Panel-type taxonomy | Vue/PrimeVue frontend (we use `nexus-ui`) |
| Multi-tenant RLS migration pattern | `Engine`/binary structure |

---

## 11. Risks / Spikes

1. **Collector sink + bounded-stream termination** — does a finite `Stream` cleanly complete
   and let us drain results? (M0 settles this.)
2. **Per-query latency** — spinning a `Stream` per panel request must be interactive-fast.
3. **Multi-tenancy** — ArkFlow engine is untenanted; isolation is the control plane's job +
   Postgres RLS. Never let a stream read another tenant's connection config.
4. **Build weight** — ArkFlow pulls protoc + PyO3 (Python UDF). Disable the Python feature
   if unused to lighten builds.
5. **ArkFlow version skew (cancellation)** — `Stream::run(cancellation_token)` + the
   `tokio::select!` abort exist in **git HEAD only**; the crates.io release has
   `run(&mut self)` with no cancellation. Our SSE abort depends on the git version → **pin
   the git rev**, and re-verify the signature on every bump. Plugin/registry API isn't
   semver-stable.
6. **`/auth/me` payload gap** — returns `{subject, email, role}` only; nexus-api must surface
   `tenant_id` + teams + effective permissions via its own `GET /api/v1/me` for the UI's
   `usePrincipal()`/`useCan()`.
7. **nexus-ui migration debt** — current deps (Refine/recharts) ≠ target (TanStack/ECharts);
   migration must land before the FE is "on-stack" (§7).
8. **ArkFlow LICENSE** is real Apache-2.0 ✅ (unlike strata, which badges MIT but ships no
   LICENSE file — irrelevant now that strata is reference-only, but note before copying code).
9. **SSE auth mismatch** — native `EventSource` can't send a Bearer header; the live route needs
   cookie/signed-token/fetch-reader auth (§5.3). Settle in M0.5.
10. **Ad-hoc SQL execution** — raw SQL from the browser needs a read-only role, server-side
    timeout/row/byte caps + forced `LIMIT`, no DDL/DML, cancellation, per-tenant filtering for
    *shared* datasource DBs, and quotas — none of it trusted from the request (§5.2).
11. **Collector OOM** — buffer-then-serialize must be bounded with caps + backpressure, and large
    results paged/streamed (§5.1).
12. **RLS mechanics with pooling** — `SET LOCAL app.tenant_id` per transaction under a
    non-`BYPASSRLS`, non-owner role + `FORCE ROW LEVEL SECURITY`; a bare pooled query with no
    surrounding tx leaks the previous tenant's context. Prove it with cross-tenant pool-reuse
    tests (§4).
13. **Extension = remote code execution** — loading `remoteEntry.js` runs trusted code in the
    user session; needs allowlist + checksum/signature + CSP + capability limits on
    `StarterClient` before any out-of-repo remote loads (§7, OVERVIEW §7).
14. **Stream lifecycle + single-node scale** — registry key must be (spec+datasource+tenant+perm)
    not "source"; define slow-subscriber/`Lagged`, reconnect/`Last-Event-ID`, heartbeat, teardown.
    In-process broadcast ⇒ **live fan-out is single-node for v1** (shared bus needed to scale out).
15. **Secrets/key management** — envelope encryption, rotation, redaction, decrypt boundary, audit
    (§4). "Encrypted" is named, not designed.
16. **Operational basics missing** — audit log, query history, rate limits, metrics/tracing for
    nexus-api itself, backup/restore expectations, and a **cross-crate migration strategy** (three
    crates own tables in one DB — who runs/orders migrations, and are there cross-crate FKs?).
17. **Strategic — ArkFlow risk is front-loaded.** ArkFlow's real value is *streaming connectors*
    (Modbus/MQTT/Kafka), which arrive at M3 — yet M0–M2 (Postgres query + dashboards) pay the
    ArkFlow tax: the request/response-over-streaming impedance mismatch, the Collector-sink
    workaround, and a hard dep on an *unreleased git rev* with a non-semver registry (Risk #5).
    The stated fallback for those milestones is "DIY on DataFusion + sqlx" — which is also what
    DataFusion does natively. **Open decision:** run M0–M2 directly on DataFusion+sqlx and bring
    ArkFlow in at M3 when live connectors justify the coupling, keeping the engine seam (Risk #1)
    off the critical path for the first working slice.

---

## 12. Tech Reference

| Concern | Choice |
|---|---|
| Engine core | **ArkFlow** (arkflow-core + arkflow-plugin), cargo git dep |
| Query engine | DataFusion (via ArkFlow), Arrow `RecordBatch` |
| Control plane | `nexus-api` — Rust + Axum |
| DB access | `sqlx` (Postgres/TimescaleDB) |
| AuthN / users / teams | `starter-auth-users` (`Principal`) |
| AuthZ | `starter-authz` (in-repo policy engine; OpenFGA only if relationship depth demands) |
| Multi-tenancy | Postgres RLS bound to `Principal.tenant_id` (enforced under runtime role) |
| Realtime | SSE (Axum) ← custom ArkFlow SSE output sink |
| Query I/O | custom ArkFlow Collector output sink → Arrow→JSON |
| Datasource extensibility | ArkFlow input-plugin registry (`register_input_builder`) |
| Frontend | React **19** + React Router + shadcn/ui + **TanStack Query** + **zustand** (+ rhf/zod) |
| Dashboard grid | react-grid-layout (maintenance — spike dnd-kit) |
| Charts | ECharts (Phase 1); uPlot/AG Grid/xterm.js as needed |
| FE extensions | In-repo runtime-federation SDK (`@nube/starter-ext-ui` + `-sdk-ts`), Vite lib-mode, not webpack MF — see `nexus-ui/OVERVIEW.md` §7 |
| Shared FE singletons | React, react-dom, `@tanstack/react-query`, zustand (matching-majors, hard refusal on mismatch) |

---

## 13. Ingestion: saved flows in v1, heavy ETL out

**Draw the line clearly** (the topology's `FlowManager` + `flows` table and this section must
agree): **saved flows are an in-scope v1 feature** — a flow is a long-running ArkFlow `Stream`
(e.g. weather→Postgres, NEXUS_TOPOLOGY §3) the `FlowManager` runs inside nexus-api, owned by a
tenant, configured (not coded) from built-in input/output plugins. That's "light ingestion,"
and it's core because live panels and connectors already need the same runner.

**Out of scope for v1 is *heavy* ETL/replication:** batch pipelines, CDC, and Postgres→lake
replication. Noted options for that layer (not in the locked stack): **ArkFlow as a separate
ingest service** at larger scale; **Supabase ETL** (Rust, Apache-2.0) for Postgres→lake
(DuckLake/BigQuery) — premature until dashboards outgrow direct Timescale querying. If flows
turn out to need their own scheduler/scaling, promote `FlowManager` to a separate service then.

---

*Update this doc when a locked decision changes. Backend crate: `nexus-api`. Frontend:
`nexus-ui`. Engine: ArkFlow (vendored as a pinned dependency).*
