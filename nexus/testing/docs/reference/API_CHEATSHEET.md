# API Cheatsheet — curl-Ready

> Verified: **WORKING on nexus-rewrite, 2026-06-10** (auth flow + flows + query
> exercised end-to-end). **`backend/openapi.json` is the source of truth** for
> request/response shapes — when a snippet fails, grep openapi.json and fix here.

**Auth is a cookie jar + CSRF token, NOT a bearer.** (This is the canonical flow
used by `nexus/e2e-test.sh`.) Login sets a session cookie in the jar and returns
a `csrf_token`; subsequent calls send the jar with `-b "$JAR"` and **mutations**
(POST/PUT/DELETE) add `-H "X-CSRF-Token: $csrf"`. `GET`s need only the jar.

```bash
export BASE=http://127.0.0.1:4780
export JAR=$(mktemp)
```

---

## Auth / identity (`/auth/*`)

```bash
# login → sets session cookie in $JAR, returns {csrf_token}
login=$(curl -s -c "$JAR" -X POST $BASE/auth/login \
  -H content-type:application/json \
  -d '{"email":"admin@nexus.local","password":"change-me-admin"}')
csrf=$(printf '%s' "$login" | sed -n 's/.*"csrf_token":"\([^"]*\)".*/\1/p')
echo "csrf=$csrf"

# who am I — UUID subject + role + tenant + teams
curl -s -b "$JAR" $BASE/api/v1/me | jq
# → {"subject":"…","role":"admin","tenant_id":"nexus","teams":["hvac-ops"],"scopes":[]}

# human identity (email) lives at /auth/me
curl -s -b "$JAR" $BASE/auth/me | jq

# unauthenticated /api/v1/me returns 401 (auth guard)
# tenants CRUD, tenant-user membership → /auth/tenants, /auth/tenants/{id}/users
```

A mutation helper used throughout:

```bash
post() { curl -s -b "$JAR" -X POST "$BASE$1" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d "$2"; }
```

---

## Flows + ingest

Node config envelope: each of `input` / `output` is `{"type":"<kind>", …config}`
(discriminator is **`type`**); `pipeline` is a JSON **array** of processor nodes.
`node-types` lists kinds + their JSON-schema (and reports the field as `kind`).

```bash
curl -s -b "$JAR" $BASE/api/v1/flows | jq                                   # list
curl -s -b "$JAR" $BASE/api/v1/flows/node-types | jq                        # kinds + schemas
post /api/v1/flows "$(cat flow.json)" | jq                                  # create (→ {id,…})
post /api/v1/flows/$FLOW/start '' | jq '.running, .metrics.last_error'      # start
post /api/v1/flows/$FLOW/stop  '' | jq                                      # stop
# dry-run, import/export → /api/v1/flows/dry-run, /{id}/export, /import

# push JSON into a running http_ingest flow
post /api/v1/ingest/$FLOW '{"value":1}'
```

Flow metrics (`batches_in`, `rows_written`, `channel_depth`, `write_errors`,
`last_error`) come back on the flow detail/list — your ingest health signal.
Stop→start resets them (single-node, in-process).

---

## Query (READ-ONLY)

`/api/v1/query` runs in a **read-only transaction** — `SELECT` only. DDL/DML
(`CREATE`/`INSERT`/`DROP`) returns `invalid_input: cannot execute … in a
read-only transaction`. Create tables via psql against the datasource DB.

```bash
post /api/v1/query '{"sql":"SELECT count(*) AS n FROM telemetry_raw"}' | jq
# → {"columns":[…],"rows":[{"n":200}],"stats":{…}}
# query history, query-kinds CRUD also under /api/v1
```

---

## Dashboards / panels / variables / nav / tags

```bash
# dashboards
curl -s $BASE/api/v1/dashboards -b "$JAR" | jq
curl -s $BASE/api/v1/dashboards -X POST -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d '{"name":"Energy","slug":"energy"}' | jq

# panels (children of a dashboard) — datasource_id, sql, viz, layout
curl -s $BASE/api/v1/panels -X POST -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d @panel.json | jq

# variables (per dashboard slug)
curl -s $BASE/api/v1/dashboards/energy/variables -b "$JAR" | jq
curl -s $BASE/api/v1/dashboards/energy/variables -X POST -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d @var.json | jq

# nav tree (the access-grant unit)
curl -s $BASE/api/v1/nav -b "$JAR" | jq
curl -s $BASE/api/v1/nav -X POST -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" \
  -d '{"title":"Energy","target":{"kind":"dashboard","dashboardId":"<uuid>"}}' | jq

# tags (feed context/tag variables)  ⚠️ no entity check on write today
curl -s $BASE/api/v1/tags/dashboard/$DASH_ID -b "$JAR" | jq
curl -s $BASE/api/v1/tags/dashboard/$DASH_ID -X PUT -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d '{"region":"north"}'
```

---

## Insights / alerts

```bash
# insight preview (test a script without saving)
curl -s $BASE/api/v1/insights/preview -X POST -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d @insight.json | jq
curl -s $BASE/api/v1/insights/functions -b "$JAR" | jq   # primitive catalog
curl -s $BASE/api/v1/insights -X POST -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d @insight.json | jq              # save

# alert rules + channels + silences
curl -s $BASE/api/v1/alerts/rules -b "$JAR" | jq
curl -s $BASE/api/v1/alerts/rules -X POST -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d @rule.json | jq
```

---

## Authz grants

```bash
# grant viewer/editor/manager on a resource instance (dashboard or nav_node)
curl -s $BASE/v1/authz/resources/nexus.nav_node/$NODE_ID -X POST -b "$JAR" -H content-type:application/json -H "X-CSRF-Token: $csrf" \
  -d '{"subject":"<user-or-team>","role":"viewer"}' | jq
# list grantable instances (ACL view)
curl -s $BASE/v1/authz/resources/nexus.nav_node/instances -b "$JAR" | jq
```

> Exact grant request body / role names → confirm in openapi.json under
> `/v1/authz/*`. Fix here if drifted.

---

## Health / readiness

```bash
curl -s $BASE/health || curl -s $BASE/api/v1/health
```

---

## When a snippet 4xx/5xx's

1. `grep "<path>" backend/openapi.json` — confirm the path, method, and body
   schema.
2. Fix the snippet here, bump the `Verified:` line.
3. If the path is gone/renamed, that's a real drift — note it and check the
   `WS-xx` scope docs / recent commits.
