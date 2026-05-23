# authz-demo — canonical Phase 7 walkthrough

This crate is the single runnable example that exercises **every**
slice of `starter-authz` Phase 7
([`DOCS/auth/authz/SCOPE-EXT.md`](../../DOCS/auth/authz/SCOPE-EXT.md)).
Phases 1–6 still apply unchanged — the additions below are strictly
on top.

| Slice | What this demo shows | Hard rule |
|-------|----------------------|-----------|
| 7a Tenants | Two tenants seeded; cross-tenant access denies *before* role/condition | **R11**, R12 |
| 7b Teams   | One team rule (`team:hvac-ops, weather, refresh, allow`) covers every team member without per-user rows | **R13** |
| 7c Audit   | `DbDecisionSink` wired in `server.rs`; 100% denies + sampled allows; `GET /v1/authz/decisions` pages them | **R14** |
| 7d REST permission   | `block.yaml` declares `auth.permission: {resource, action}` inline; the REST adapter wires `with_permission` — zero host-side mounting | **R15** |
| 7d.2 MCP + gRPC parity | Same `auth.permission` field consumed by the MCP and gRPC adapters; deny rows tagged with `surface = mcp\|grpc\|rest` | **R15** |

The rest of this document is a single end-to-end script you can
copy-paste.

## Endpoints

| Surface | Path / id                              | Resource  | Action    | Comes from         |
|---------|----------------------------------------|-----------|-----------|--------------------|
| REST    | `GET  /reports`                        | `reports` | `read`    | server (built-in)  |
| REST    | `POST /reports`                        | `reports` | `create`  | server (built-in)  |
| REST    | `GET  /weather/forecast`               | `weather` | `read`    | `com.acme.weather` manifest |
| REST    | `POST /weather/refresh`                | `weather` | `refresh` | `com.acme.weather` manifest |
| REST    | `GET  /v1/authz/decisions`             | `audit_logs` | `read` | `starter-authz`    |
| MCP     | tool `com.acme.weather.forecast_tool`  | `weather` | `read`    | `com.acme.weather` manifest |
| gRPC    | `weather.v1.Weather/Current`           | `weather` | `read`    | `com.acme.weather` manifest |

Built-in role defaults (loaded by `default_policy: true`):

- `reader`  → `read` on every kind.
- `writer`  → `read` + `create` + `update` on every kind.
- `admin`   → everything.

`refresh` is intentionally **not** in the default action list — out of
the box no one can call `POST /weather/refresh`. The team rule grants
it to every member of `team:hvac-ops`.

## Walkthrough

```bash
# from repo root
cargo build -p starter-authz-demo
BIN=./target/debug/authz-demo
cd examples/authz-demo
rm -f authz-demo.db*

# 1. migrations (auth-users + authz + reports + tenants + teams + audit)
$BIN migrate

# 2. seed two tenants (R11 + R12)
TA=$($BIN tenant create acme   "Acme HVAC")
TB=$($BIN tenant create globex "Globex HVAC")

# 3. seed users — alice in acme (reader), bob in acme (writer),
#    carol in globex (writer)
ALICE=$($BIN user create alice@acme.test alice-secret-pw1 --role reader --tenant acme)
BOB=$(  $BIN user create bob@acme.test   bob-secret-pw1   --role writer --tenant acme)
CAROL=$($BIN user create carol@globex.test carol-secret-pw1 --role writer --tenant globex)

# 4. issue API tokens bound to the right tenant
AT=$($BIN user token $ALICE --tenant acme)
BT=$($BIN user token $BOB   --tenant acme)
CT=$($BIN user token $CAROL --tenant globex)

# 5. create the hvac-ops team in tenant acme and add bob
$BIN team create  acme hvac-ops "HVAC Ops"
$BIN team add     acme hvac-ops $BOB

# 6. seed the team rule — every hvac-ops member can refresh weather
$BIN rule create --team hvac-ops --tenant acme --resource weather --action refresh --effect allow

# 7. start the server
STARTER_AUTHZ_DECISION_SINK=db \
  $BIN serve --http-bind 127.0.0.1:8090 &
sleep 1
H=http://127.0.0.1:8090
```

### Tenant predicate (R11)

```bash
# bob in tenant=acme reads reports in his own tenant — OK
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $BT" $H/reports                # 200

# carol in tenant=globex tries to read a row owned by acme — Deny
# (cross_tenant reason; rule was never consulted)
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $CT" $H/reports/acme-1         # 403

# a token without any tenant binding hits the no_tenant_binding deny
# (issued via `--tenant *` only for super-admin; not shown here)
```

### Team predicate (R13)

```bash
# bob is a member of hvac-ops, gets the refresh via the team rule
curl -s -X POST -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $BT" $H/weather/refresh  # 200

# remove bob from the team — refresh is denied immediately on the
# next token-verify (Principal.teams is re-populated at session-mint)
$BIN team remove acme hvac-ops $BOB
curl -s -X POST -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $BT" $H/weather/refresh  # 403
```

### Audit sink (R14) + decisions endpoint

```bash
# every deny above landed in starter_authz_decisions. paginate
# (admin-only — bob is a writer, super-admin token would be needed
# in real life; for the demo we issue one):
ADM=$($BIN user create root@acme.test root-secret-pw1 --role admin --tenant '*')
AT_ADM=$($BIN user token $ADM --tenant '*')

curl -s -H "Authorization: Bearer $AT_ADM" \
     "$H/v1/authz/decisions?limit=20" | jq '.entries[] | {at, effect, reason, surface, action, kind}'
```

The sink default is `off`; setting `STARTER_AUTHZ_DECISION_SINK=db`
enables `DbDecisionSink` with bounded mpsc (default depth 4096),
100% deny retention, and sampled allows (`STARTER_AUTHZ_DECISION_ALLOW_SAMPLE`,
default 100). Overflow drops with `tracing::warn { dropped_count }`
— `check()` never blocks. Retention defaults to 90 days
(`STARTER_AUTHZ_DECISION_RETAIN_DAYS`) and runs hourly.

### Extension permission gate (R15)

[`extensions/com.acme.weather/block.yaml`](extensions/com.acme.weather/block.yaml)
declares the `permission:` block inline on the REST entries, on one
MCP tool entry, and on one gRPC method entry. There is **no
host-side hand-mounting**:

```yaml
contributes:
  rest:
    - id: com.acme.weather.refresh
      method: POST
      path: /weather/refresh
      auth:
        permission: { resource: weather, action: refresh }
  tools:
    - id: com.acme.weather.forecast_tool
      auth:
        permission: { resource: weather, action: read }
  grpc:
    - id: com.acme.weather.forecast_rpc
      service: weather.v1.Weather
      method:  Current
      auth:
        permission: { resource: weather, action: read }
```

Each adapter wraps its dispatch in `with_surface("rest" | "mcp" | "grpc", …)`
so the audit row carries the surface label — `GET /v1/authz/decisions`
above exposes it.

A manifest declaring `permission: { resource: doesnt_exist, … }`
fails at `rest_router::build` time with
`RestBuildError::UnknownResource` (symmetric with `UnknownRole`):
that one extension refuses to mount and the rest of the host comes up.

## Where to look in the code

- [`src/server.rs`](src/server.rs) — router composition. Note the
  layer order: `with_principal` outermost, then the policy engine
  extension, then per-route `with_role` → `with_scope` →
  `with_permission` → handler. Also wires `DbDecisionSink` and
  `spawn_retention`.
- [`src/reports.rs`](src/reports.rs) — built-in resource, in-handler
  ownership check using `check_or_deny(...)`.
- [`src/weather.rs`](src/weather.rs) — extension dispatcher closure.
  Hand-mounted routing is gone; `BuiltinRestDispatcher` is what the
  adapter calls.
- [`src/admin.rs`](src/admin.rs) — `tenant`, `team`, `user`, `rule`,
  `grant`, `revoke` CLI sub-commands. All mutate the DB directly.
- [`extensions/com.acme.weather/block.yaml`](extensions/com.acme.weather/block.yaml)
  — the manifest. Look at the three `auth.permission` blocks.

## Caveats and deferred work

- `grant` / `revoke` / `rule create` go straight to the DB. The
  running server's engine cache is reloaded by the **CLI** process,
  not the **server** process, so policy changes only take effect
  after restarting the server. A production deployment uses the
  `/v1/authz/rules` admin REST routes instead, which call
  `engine.reload()` in the server process automatically.
  Multi-instance cache invalidation is **deferred** — see
  SCOPE-EXT.md §6 "Open questions".
- `DELETE /v1/tenants/{id}` is **deferred** to an ADR
  (`ADR-tenant-deletion` — see SCOPE-EXT.md §6).
- Tenant predicate query pushdown is **deferred** — the engine still
  evaluates the predicate in Rust; SCOPE-EXT.md §6 captures this.
- The condition mini-language only ships `contains` for arrays; an
  `intersect` operator is **deferred** (SCOPE-EXT.md §6).
- Audit log size scaling beyond 90 days × moderate volume relies on
  the retention task — large-scale partitioning is **deferred**
  (SCOPE-EXT.md §6).
