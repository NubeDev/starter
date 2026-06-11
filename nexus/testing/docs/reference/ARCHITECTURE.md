# Nexus Architecture — Grounded Map for Testing

> Verified: nexus-rewrite tip on 2026-06-10. **Re-grep before trusting any
> file:line below** — citations rot. If a claim has drifted, fix it here first,
> bump this line, then continue.

This is the factual substrate the rest of the testing docs stand on. It is
deliberately terse and citation-heavy: when a runbook step surprises you, come
here to confirm how the system *actually* behaves.

---

## 1. Processes, ports, storage

| Component | Binary / image | Default endpoint | Notes |
|-----------|---------------|------------------|-------|
| Control plane API | `nexus-api` (`backend/crates/nexus-api`) | `127.0.0.1:4780` | `NEXUS_BIND` overrides |
| Metadata DB | Postgres | `NEXUS_METADATA_URL` | control-plane tables, RLS |
| Datasource DB | Postgres | `NEXUS_DATASOURCE_URL` | telemetry / query target |
| MQTT broker | `eclipse-mosquitto:2` | `1883` | no-auth for tests |
| NATS broker | `docker/testing-nats.Dockerfile` (`nats:2-alpine`) | `4222` (mon `8222`) | no-auth |
| Zenoh router | `docker/testing-zenoh.Dockerfile` (`eclipse/zenoh`) | `7447` (api `8000`) | no-auth |
| Data generator | `testing/datapump` | — | pumps MQTT / Zenoh |

Required backend env: `NEXUS_METADATA_URL`, `NEXUS_DATASOURCE_URL`,
`NEXUS_MASTER_KEY` (32 bytes), `NEXUS_STREAM_TOKEN_KEY` (≥32 bytes). Kinds dirs:
`NEXUS_KINDS_DIR`, `NEXUS_DATASOURCE_KINDS_DIR`. Migrations apply on boot.

Seeding: `make seed` → `seed-admin` bin (admin tenant+user+grants from
`ADMIN_EMAIL`/`ADMIN_PASSWORD`); `make seed-sim` → `nexus-seed` bin (writes
`sim_hvac`/`sim_energy`/`sim_door`, `SIM_ROWS` rows each).

---

## 2. Flows / ingest

- Table `nexus_flows` (`nexus-store/migrations/nexus/0003_flows.sql`): three JSONB
  columns `input` / `pipeline` / `output`, tenant-scoped, RLS. No DB schema on the
  JSON — validated when the flow starts (`nexus-engine/src/flow/manager.rs`).
- **Node envelope (verified 2026-06-10):** each `input`/`output` is a JSON object
  `{"type":"<kind>", …config}` — the discriminator key is **`type`**
  (`flow/manager.rs` `node_type()` reads `node["type"]`). `pipeline` is a JSON
  **array** of processor nodes. `node-types` API reports each kind under a `kind`
  field, but the flow config itself uses `type`.
- **The `zenoh` source needs a cargo feature.** It's registered only under
  `#[cfg(feature = "zenoh")]` in `native_registry.rs`. `nexus-api` exposes a
  `zenoh` feature that forwards to `nexus-engine/zenoh`; run with
  `make dev-be FEATURES=zenoh` (or `cargo run --features zenoh`). Without it the
  `zenoh` node kind is absent and a flow start fails.
- **postgres sink config:**
  `{"type":"postgres","uri":"<connstr>","table":"<name>","create":true,"primary_key":[…],"on_conflict":"error|nothing|upsert"}`
  — `uri` is the connection string (not a datasource_id). The sink binds **typed
  by the stream's Arrow schema** (string-under-`Timestamp` → `timestamptz`,
  int/float/bool native) and **auto-creates the table** from that schema
  (`create:true` default) with the declared column types + optional PK. So the
  declared `json_to_arrow` schema is the one source of truth for parse, storage
  DDL, and binding. Verified end-to-end: zenoh → json_to_arrow (declared schema) →
  auto-created `telemetry_typed` (timestamptz/double precision/PK), 200/200 rows.
  (`nexus-engine/src/sink/postgres_sink.rs` + `pg_insert.rs`.)
- Enabled flows auto-start on boot. Each flow is a bounded-channel pipeline with
  backpressure; live metrics `batches_in` / `rows_written` / `channel_depth` /
  `write_errors` (`nexus-spi/src/dto/flow/shared.rs`).
- **Sources** (`nexus-engine/src/source/`): `http_ingest`, `http_poll`, `zenoh`
  (feature-gated, OFF by default), `interval`, `generate`, `sim`, `memory`.
  **There is no MQTT source.**
- **Processors** (`nexus-engine/src/processor/`): `json_to_arrow`,
  `declared_schema`, `sql`.
- **Sinks** (`nexus-engine/src/sink/`): `postgres`, `datasource`, `collector`,
  `drop`, `stdout`, `sse`, `broadcast_store`. Postgres sink uses bind-param
  inserts (never string concat).
- HTTP ingest: `POST /api/v1/ingest/{flow_id}` enqueues raw JSON (object or array)
  into a running `http_ingest` flow's channel (`nexus-api/src/routes/ingest/push.rs`).
- Zenoh source: opens a session, subscribes to a `key_expr`, forwards each JSON
  sample as a carrier doc for `json_to_arrow`. Modes `client` (router at
  `endpoints`) or `peer` (in-process mesh). At-most-once delivery
  (`nexus-engine/src/source/zenoh.rs`).
- MQTT exists only as a **datasource kind** (connection config), not an ingest
  source: `nexus-api/datasource-kinds/mqtt_config.json`.

**Implication for testing:** to get datapump's MQTT stream into a flow you need
either (a) datapump's Zenoh transport into the `zenoh` source, or (b) an
MQTT→HTTP bridge that POSTs to `/api/v1/ingest/{flow_id}`. See
[features/FLOWS_MQTT_INGEST.md](../features/FLOWS_MQTT_INGEST.md).

---

## 3. Insights & alerts

- **Insights** — `nexus_insights` table: `script` (Rhai) + advisory
  `params_schema`. A post-query transform over the result frame, sandboxed; binds
  `df` + `params`. Composes vectorized primitives (`resample`, `zscore`,
  `anomalies`, …) executed by DataFusion — no row loops; caps stop result growth.
  Errors classified `Compile` / `LimitExceeded` / `Runtime`
  (`nexus-insights/src/run.rs`). Attached to a query via `InsightRef` (stored id,
  extension name, or inline script). API: `POST /insights`, `PUT /insights/{id}`,
  `POST /insights/preview`, `GET /insights/functions`.
- **Alerts** — `nexus_alert_rules` table: single-condition (`query`/`op`/
  `threshold`) or multi-condition (`conditions[]` + `combinator` /
  `no_data_policy` / `exec_error_policy`). Cadence `interval_secs`, dwell
  `for_secs`. Fire to `channel_ids[]` (channels hold delivery creds). API:
  `GET|POST /alerts/rules`, `PUT|DELETE /alerts/rules/{id}`, plus channels &
  silences. (Evaluator scheduler not re-confirmed in this sweep — verify before
  relying on auto-fire timing.)

---

## 4. Dashboards / pages / nav / variables

- `nexus_dashboards` (id, tenant, mutable `slug`, name); panels in
  `nexus_panels` (datasource_id, sql, viz, layout JSON). Route by slug `/d/:slug`.
- **Nav** — `nexus_nav_nodes` (self-ref `parent_id`, `title`, `sort_order`,
  `target` tagged union, `context`, icon, accent). Targets: `group` (header),
  `dashboard` (mount a page), `route` (static app page). Deleting a dashboard
  reverts its nodes to `group`. **Grants attach to nav nodes** — the new access
  unit. API: `GET|POST /nav`, `PUT|DELETE /nav/{id}`.
- **Variables** — `nexus_dashboard_variables`: kinds
  `constant|custom|query|datasource|interval|textbox|context`. Resolve in order
  (constants → custom/datasource/interval → query by dependency → built-ins
  `$__dashboard`/`$__user`/`$__from`/`$__to`). Inject as bound `$N` args, never
  inlined. `context` kind reads nav/url/tag/values (read-only). URL deep-link:
  `?var-region=Site-A`. API per dashboard: `GET|POST /dashboards/{slug}/variables`,
  `PUT|DELETE …/{id}`.
- **Tags** — `nexus_tags` (entity_type, entity_id, key, value). Feed
  `context`/`tag` variable sources. ⚠️ Write path is tenant-only with **no entity
  existence/edit check** (`routes/tags/set.rs`) — a known authz gap; a tag write
  on a bogus id currently succeeds.
- **Page context** — assembled from nav/url/tags/values with explicit precedence;
  read-only input to variable resolution, not a persistence layer. Query cache key
  bumps on context change (`varRevision`).

See [WS-13_NAV_AND_CONTEXT.md](../../../docs/scope/nextgen/WS-13_NAV_AND_CONTEXT.md)
and [WS-02_VARIABLES_AND_TEMPLATING.md](../../../docs/scope/nextgen/WS-02_VARIABLES_AND_TEMPLATING.md).

---

## 5. Users / teams / auth / RLS

- Identity is mounted from `starter-auth-users` on `/auth/*`: login/logout/reset,
  tenant CRUD, tenant-user membership (`nexus-api/src/identity.rs`). Principal
  carries `tenant_id` + teams, verified per request.
- Authorization via `starter-authz` `DbPolicyEngine`, `default_policy = true`:
  tenant admins reach everything; non-admins get access only via explicit
  per-resource grants. Grant CRUD at `/v1/authz/resources/{kind}/{id}`.
- Instance providers register grantable resources: `nexus.dashboard`,
  `nexus.nav_node`. Nav node is the new grant target replacing per-dashboard
  sharing.
- **RLS is database-enforced**: each table has a Postgres policy scoped to the
  `app.tenant_id` setting. Set the wrong tenant → rows vanish. This is the #1
  testing gotcha; always confirm which tenant your token and your writes use.

---

## 6. API surface (path prefixes under `/api/v1/`)

`agents`, `ai/assist`, `alerts`, `audit`, `dashboards`, `panels`, `datasources`,
`flows`, `folders`, `ingest/{flow_id}`, `insights`, `me`, `nav`, `query`,
`query-kinds`, `streams`, `tags`, `undo`, `redo`, `variables`; plus `/auth/*`
and `/v1/authz/*`. Full DTOs in `backend/openapi.json` — the FE contract; treat
it as the source of truth for request/response shapes.
