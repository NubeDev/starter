# Feature: Database Access — Datasource Explorer & the Nexus-DB Inspector

> Verified: nexus-rewrite, 2026-06-11. Covers the two ways SQL reaches a Postgres
> through the API — the **datasource explorer** (`POST /query`,
> `POST /datasources/:id/query`) and the new admin **nexus-DB inspector**
> (`POST /api/v1/nexus-db/query`) — and exactly which database each one hits.

There are **two physically separate connection pools** in `AppState`
(`backend/crates/nexus-api/src/state.rs`), and the whole story is about which one
a request lands on:

| Pool | Env var | Holds | Reached by |
|------|---------|-------|------------|
| `state.metadata` | `NEXUS_METADATA_URL` | The control plane — users, datasources, dashboards, flows, agents. Tenant tables under RLS. | `POST /api/v1/nexus-db/query` (admin) + every internal store call |
| `state.datasource` (+ per-datasource pools) | `NEXUS_DATASOURCE_URL` / registered datasource rows | The *data* a tenant queries — telemetry, sim tables, external DBs | The datasource explorer (`POST /query`, `POST /datasources/:id/query`) |

> ⚠️ **Dev gotcha:** the `Makefile` points **both** env vars at the *same*
> `make db` Postgres (`:4770`). So in dev the explorer's "default datasource" and
> the metadata DB are physically one database — the explorer can incidentally see
> nexus's own tables. In production these are different DSNs and that overlap
> disappears. The two pools are always logically separate regardless.

---

## A — Datasource Explorer (existing)

UI: the **Explore** page (`ui/src/features/query-editor/Explore.tsx`). Pick a
datasource, write SQL, Run. Routes:

- `POST /api/v1/query` — the dev single-datasource shortcut / federation path
  (`state.datasource`).
- `POST /api/v1/datasources/{id}/query` — a specific registered datasource
  (resolved to a pool via `DatasourcePools::get_or_connect`, gated by an authz
  `view` grant on the datasource).

This **never touches `state.metadata`**. To query anything it must resolve a
datasource *row* for the caller's tenant; the metadata DB is not a registered
datasource, so there is no datasource id that maps to it.

---

## B — Nexus-DB Inspector (new — `POST /api/v1/nexus-db/query`)

A read-only window into the **control-plane DB itself** (`state.metadata`) — the
same database `make db` serves. For inspecting platform internals (who's in this
tenant, what datasources/flows/agents exist) without a `psql` shell.

### Request / response
```http
POST /api/v1/nexus-db/query
Authorization: Bearer <admin-token>
Content-Type: application/json

{ "sql": "SELECT id, name FROM nexus_agents" }
```
Returns the standard `QueryResponse` (`{ columns, rows, stats }`) — the same
shape the Explore result grid renders.

### Guardrails (three independent axes)
| Axis | Behaviour | Enforced by |
|------|-----------|-------------|
| **Admin only** | A non-admin principal is refused `403` before any SQL runs | `principal.role != Role::Admin` check in the handler |
| **Tenant-scoped** | The tx binds `app.tenant_id`, so RLS filters every row to the caller's tenant — an admin sees only **their** tenant's rows | `set_config('app.tenant_id', …, true)` in `run_query_tenant_ro` |
| **Read-only + capped** | `SET TRANSACTION READ ONLY` (writes/DDL rejected by Postgres), plus the standard `statement_timeout` + 10k-row / 16MB caps | `run_query_tenant_ro` + `state.guards` |

### Status codes
| Code | When |
|------|------|
| 200 | Rows returned (`stats.truncated = true` if a cap stopped it) |
| 400 | Malformed SQL, or a write/DDL rejected by the read-only tx |
| 401 | No principal |
| 403 | Authenticated but not an admin, or no tenant binding |

### Files
| Layer | Path |
|-------|------|
| Handler | `backend/crates/nexus-api/src/routes/nexus_db/query.rs` |
| Router | `backend/crates/nexus-api/src/routes/nexus_db/mod.rs` (merged in `routes/mod.rs`) |
| Store fn | `backend/crates/nexus-store/src/query/run.rs` → `run_query_tenant_ro` |
| OpenAPI | registered in `backend/crates/nexus-api/src/openapi.rs` |

---

## How to test (stack up: `make dev-be`, admin seeded)

```bash
# Grab an admin bearer token first (login / seeded admin), then:
curl -s localhost:4780/api/v1/nexus-db/query \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"sql":"SELECT id, name, backend, model FROM nexus_agents"}' | jq

# A write is rejected by the read-only tx (expect 400):
curl -s -o /dev/null -w '%{http_code}\n' localhost:4780/api/v1/nexus-db/query \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"sql":"DELETE FROM nexus_agents"}'

# A non-admin token is refused (expect 403).
```

Things to confirm:
- A `SELECT` against a metadata table (`nexus_agents`, `nexus_datasources`,
  `nexus_flows`, …) returns this tenant's rows.
- A second tenant's admin sees a **disjoint** set of rows from the same query
  (RLS isolation).
- `DELETE` / `UPDATE` / `CREATE TABLE` all return 400, not 200.

## Known gaps / to add
- **No UI yet** — backend endpoint only. A "Nexus DB" tab in Explore pointed at
  this route is the obvious follow-up.
- **Tenant-scoped only.** It deliberately cannot read across tenants: the
  `nexus_runtime` role is `FORCE ROW LEVEL SECURITY`, so a true cross-tenant
  inspector would need a different DB role, not just a GUC change.
- No e2e test in the suite yet — add one asserting the 403 (non-admin), 400
  (write), and tenant-isolation cases above.
