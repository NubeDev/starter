# Nexus — Concrete Topology

> Companion to [`NEXUS.md`](./NEXUS.md). Shows the full stack with ArkFlow as a concrete
> wiring diagram, plus walkthroughs for: a Postgres data connector, where users/dashboards
> are stored, an example ingestion plugin (weather → Postgres), and dashboard/team access.
> Status mirrors NEXUS.md: **architecture locked; wire-level contracts pending M0/M1.**

## Storage decision (TL;DR)

- **Control-plane store = Postgres, NOT SQLite.** We rely on **Postgres RLS** for tenant
  isolation (SQLite has none), need multi-user concurrent writes (SQLite is single-writer),
  and `starter-auth-users`/`starter-authz` ship Postgres migrations. SQLite is a **dev /
  single-node** option only — and then tenancy must be enforced purely in app code.
- **Two logically separate Postgres roles:**
  1. **Metadata DB** (`nexus`) — nexus-api's own data: users, teams, grants, dashboards,
     panels, datasource configs, flows, alerts.
  2. **Data-source DB(s)** (e.g. `rubix`/Timescale) — what panels *query*. The first
     connector. May be the same physical server, different database.

---

## Full stack

```
╔══════════════════════════════════════════════════════════════════════════════╗
║  BROWSER — nexus-ui (React 19 + shadcn + TanStack Query + zustand)             ║
║   base: nexus-ui/OVERVIEW.md · federation host (@nube/starter-ext-ui)          ║
║   Dashboard pages · panel canvas · query editor · LIVE panels (EventSource)   ║
╚════════════════════════════════════╤═════════════════════════════════════════╝
         REST /api/v1/* (Bearer)  +  SSE /api/v1/streams/:id (cookie/signed-token — not Bearer)
╔════════════════════════════════════▼═════════════════════════════════════════╗
║  nexus-api  (Rust / Axum — the CONTROL PLANE)                                 ║
║                                                                               ║
║  ┌─ Identity ────────────┐  ┌─ Product API ─────────┐  ┌─ Engine bridge ───┐ ║
║  │ starter-auth-users    │  │ /datasources          │  │ QueryRunner       │ ║
║  │  → Principal          │  │ /dashboards /panels   │  │  1-shot Stream    │ ║
║  │ starter-authz         │  │ /alerts /templates    │  │  → Collector sink │ ║
║  │  → grants (team→page) │  │ /query  /streams      │  │ LiveRunner        │ ║
║  └───────────┬───────────┘  └───────────┬───────────┘  │  → SSE sink       │ ║
║              │                          │              │ FlowManager       │ ║
║              │ reads/writes             │ reads/writes │  → weather, etc.  │ ║
║              ▼                          ▼              └─────────┬─────────┘ ║
║  ┌──────────────────────────────────────────────┐               │           ║
║  │  METADATA DB   ← Postgres (db: "nexus")  ★    │               │ embeds    ║
║  │  (NOT sqlite — needs RLS + concurrency)       │               ▼           ║
║  │  users, teams, memberships, sessions          │   ┌───────────────────┐   ║
║  │  authz_grants  (who can see which dashboard)  │   │ ArkFlow engine    │   ║
║  │  datasources   (connection configs, encrypted)│   │ (lib, git-pinned) │   ║
║  │  dashboards, panels, alerts, templates        │   │ Stream/registry   │   ║
║  │  flows         (saved ingestion pipelines)    │   │ DataFusion + Arrow │   ║
║  │  ── all rows tenant_id-scoped via RLS ──      │   └─────────┬─────────┘   ║
║  └──────────────────────────────────────────────┘             │             ║
╚════════════════════════════════════════════════════════════════│════════════╝
                                                                  │ register_*_builder()
                          ┌───────────────────────────────────────┼───────────────┐
                          │ ArkFlow INPUT plugins   ArkFlow OUTPUT plugins (+ ours) │
                          │  sql(postgres) ✦first    sql(postgres)                  │
                          │  http (weather)          collector (→ JSON, ours)       │
                          │  mqtt / kafka / modbus    sse       (→ browser, ours)   │
                          └───────────────────────────────────────┬────────────────┘
                                                                  ▼
   ┌──────────────────────────────┐                 ┌──────────────────────────────┐
   │ DATA-SOURCE DB  ✦ FIRST       │                 │  External APIs / brokers     │
   │ Postgres / TimescaleDB        │◀── weather ────│  weather API, MQTT, Kafka…    │
   │ (db: "rubix": samples, etc.)  │   ingest writes └──────────────────────────────┘
   │ ── panels QUERY this ──       │
   └──────────────────────────────┘
```

`★` = metadata store · `✦` = first data connector

---

## 1. Postgres as the first data connector

A **datasource** row in the metadata DB:
`{ id, type:"sql", driver:"postgres", url:<encrypted>, tenant_id }`. A panel query runs
through ArkFlow's built-in `sql` input → DataFusion → the **Collector sink** → JSON:

```
POST /api/v1/datasources/:id/query  { "sql": "SELECT ts, value_num FROM samples WHERE ..." }
   │  authz: can Principal read this datasource?  +  tenant scope
   │  guards (NEXUS §5.2): read-only role · stmt timeout · forced LIMIT · row/byte cap · no DDL/DML
   ▼
   Stream{ input: sql(postgres, url)=<the user query, pushed down>, pipeline:[], output: collector }
   ▼  drain RecordBatch (bounded) → Arrow→JSON
   200 [ {ts, value_num}, ... ]   →  ECharts panel
```

> The user's SQL is the **input** query (pushed to Postgres so `WHERE`/`LIMIT` run *in* the DB —
> see NEXUS §5.2 "two-layer SQL"); the DataFusion pipeline is for non-SQL/cross-source shaping.
> On a datasource DB **shared across tenants**, inject a per-tenant predicate — datasource
> ownership alone does not isolate rows inside a shared DB (data-side has no RLS).

---

## 2. Where users / dashboards live → metadata Postgres (`nexus`), not SQLite

| Table group | Holds | Owner crate |
|---|---|---|
| `users, teams, memberships, sessions` | identity | starter-auth-users |
| `authz_grants` | team/user → dashboard/page perms | starter-authz |
| `datasources` | connection configs (secrets **envelope-encrypted** — key mgmt/rotation/redaction/audit per NEXUS §4) | nexus-api |
| `dashboards, panels, alerts, templates` | the product | nexus-api |
| `flows` | saved ingestion pipelines (e.g. weather) | nexus-api |

All rows carry `tenant_id`; **Postgres RLS** enforces isolation — under a non-`BYPASSRLS`,
non-owner runtime role with `FORCE ROW LEVEL SECURITY` and `SET LOCAL app.tenant_id` bound
**per transaction** (NEXUS §4 RLS mechanics; pool-reuse leak tests are part of the contract).
Three crates own tables in this one DB — migration ordering/strategy is an open item.

---

## 3. Example plugin — weather every 15 min → Postgres

An ArkFlow **flow** (a saved Stream) the `FlowManager` runs continuously. This is the
*ingestion* direction (data in motion):

```
flow "weather-sync"  (row in nexus.flows, owned by a tenant):
  input:                                   ┌── runs forever inside nexus-api ──┐
    type: http                             │ poll every 15m → JSON → reshape   │
    url:  https://api.weather.com/...      │ → INSERT into Postgres            │
    interval: 15m                          └───────────────────────────────────┘
  pipeline:
    - type: sql                # DataFusion: pick fields, stamp ingest time
      query: "SELECT city, temp_c, humidity, now() AS ts FROM flow"
  output:
    type: sql                  # ArkFlow's built-in Postgres sink
    driver: postgres
    url:   <data-source Postgres>     # db "rubix", table weather_readings
    table: weather_readings
```

`weather_readings` then becomes a **queryable data source** — a panel charts it.

**Flow vs custom plugin:** the above uses built-in `http` + `sql` plugins — *zero new code*.
If you want a reusable `type:"weather"` connector, wrap it as a **custom Input plugin** via
`register_input_builder("weather", …)`. Rule of thumb: start with a flow; promote to a
plugin only when reused.

> **Scope:** saved flows like this are **in v1** (light ingestion via an ArkFlow `Stream`,
> tenant-owned, config-not-code). This is consistent with — not contradicted by — NEXUS §13:
> *heavy* batch ETL / CDC / lake replication stays out of v1. If `FlowManager` later needs its
> own scheduler/scaling, promote it to a separate service then.

---

## 4. Dashboard pages + user/team access

```
team "ops"  ──member──►  alice (tenant=acme)
   │
   └─ grant{ subject: team:ops, action: view, resource: dashboard:<uuid> }   ← immutable id, not slug
                                   │
dashboard <uuid> (slug "plant-1", tenant=acme) ─┤  panels:
   ├─ panel A → datasource: timescale-rubix · sql: "SELECT … FROM samples"
   └─ panel B → datasource: weather-pg     · sql: "SELECT … FROM weather_readings"

Request flow:  GET /api/v1/dashboards/plant-1   (slug)
   0. Resolve  → slug "plant-1" → dashboard <uuid>   (everything below uses the id)
   1. Authn    → Principal{ subject: alice, tenant: acme }
   2. Authz    → starter-authz: does alice (via team:ops) have `view` on dashboard:<uuid>?
   3. RLS      → row visible only if tenant_id = acme
   4. Render   → for each panel, run its query (§1, with §1 guards) → ECharts
```

`view` / `edit` / `admin` grants attach a team-or-user → a dashboard (or folder/page) by its
**immutable id** (the slug is a route alias only — renaming must not orphan grants/links),
checked on every read, with **RLS as defense-in-depth**.

---

## Notes / open items (track against NEXUS.md)

- **ArkFlow git-pin required** for `Stream::run(cancellation_token)` (the SSE abort path) —
  not in the crates.io release.
- **`/auth/me` gap** — nexus-api adds `GET /api/v1/me` returning `tenant_id` + teams +
  permissions (the crate's `/auth/me` returns only `{subject,email,role}`).
- **Fallback if ArkFlow is too immature** is **DIY on DataFusion + sqlx** (query) + a
  hand-rolled SSE layer — *not* Arroyo (Arroyo is a distributed platform, not embeddable).
  (See NEXUS Risk #17: since M0–M2 don't need ArkFlow's connectors, running them on
  DataFusion+sqlx and deferring ArkFlow to M3 is an open decision.)
- **SSE auth** — `EventSource` can't carry a Bearer header; live route uses cookie or a
  short-lived signed stream token (NEXUS §5.3).
- **SQL execution guards** — read-only role, server-side timeout/row/byte caps + forced LIMIT,
  no DDL/DML, per-tenant predicate on shared DBs (NEXUS §5.2). Collector must be bounded.
- **Stream registry key** = (spec + datasource + tenant + permission), not "source"; in-process
  broadcast ⇒ live fan-out is **single-node for v1** (NEXUS §5.3).
- **RLS mechanics** — non-`BYPASSRLS`/non-owner role, `FORCE ROW LEVEL SECURITY`, `SET LOCAL`
  per-tx, cross-tenant pool-reuse tests (NEXUS §4).
- **Secrets/key mgmt** — envelope encryption + rotation + redaction + audit (NEXUS §4).
- **Extension security** — allowlist/signing/CSP/capabilities before any out-of-repo remote
  loads (NEXUS §7).
- **Ops basics** — audit log, query history, rate limits, metrics/tracing, backup/restore,
  cross-crate migration ordering.
- **M0** proves §1 (Collector sink → real rows from Postgres). **M0.5** proves the SSE path.
