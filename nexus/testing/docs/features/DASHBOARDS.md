# Feature: Dashboards — Pages, Queries, Variables/Context, Nav Assignment

> Verified: **WORKING end-to-end on nexus-rewrite, 2026-06-10** — datasource →
> dashboard → line panel over typed telemetry → query variable → bind-safe
> injection → nav mount, all run live against `telemetry_typed`.
> Reference scope docs:
> [WS-13_NAV_AND_CONTEXT](../../../docs/scope/nextgen/WS-13_NAV_AND_CONTEXT.md),
> [WS-02_VARIABLES_AND_TEMPLATING](../../../docs/scope/nextgen/WS-02_VARIABLES_AND_TEMPLATING.md),
> [WS-03_QUERY_AUTHORING](../../../docs/scope/nextgen/WS-03_QUERY_AUTHORING.md).

**What we're testing:** create a dashboard page that queries the ingested
MQTT/Postgres data, parameterize it with variables, drive those variables from
nav context, and mount the page into the sidebar.

Architecture recap ([../reference/ARCHITECTURE.md §4](../reference/ARCHITECTURE.md)):
dashboards (slug + panels) · variables (7 kinds, resolve in order, inject as
bound args) · nav nodes (the mount + access unit) · tags + page context feed the
`context` variable kind.

---

## Runbook (verified)

Assumes the stack is up, you have `$JAR`/`$csrf`/`post()` from the
[cheatsheet](../reference/API_CHEATSHEET.md), and `telemetry_typed` is populated
via [FLOWS_MQTT_INGEST](FLOWS_MQTT_INGEST.md) (or use `make seed-sim` tables).

### 1. Register a datasource (panels query a datasource, not a raw URI)

A panel's `datasource_id` points at a **registered datasource**. Create a
postgres one over the telemetry DB (SQL connectors use flat fields; the password
is write-only):

```bash
DSID=$(post /api/v1/datasources \
  '{"name":"telemetry-pg","kind":"postgres","host":"127.0.0.1","port":4770,"database":"nexus","user":"nexus","password":"nexus"}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
# verify it can query the typed table (timestamp math works because it's timestamptz):
post /api/v1/datasources/$DSID/query \
  '{"sql":"SELECT kind, count(*) n, avg(value) v FROM telemetry_typed GROUP BY kind"}'
```

✅ Returns rows with a `timestamp`/`float` typed result.

### 2. Create the dashboard + a panel

Panel route is **`POST /api/v1/dashboards/{slug}/panels`** (not `/api/v1/panels`).
`viz` ∈ `line|bar|table|stat`.

```bash
DASHID=$(post /api/v1/dashboards '{"slug":"energy","name":"Energy & Water"}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')

post /api/v1/dashboards/energy/panels "$(python3 -c 'import json,os;print(json.dumps({
  "title":"Avg value by kind","datasource_id":os.environ["DSID"],
  "sql":"SELECT date_trunc('second',\"timestamp\") t, kind, avg(value) v FROM telemetry_typed GROUP BY 1,2 ORDER BY 1",
  "viz":"line"}))')"
```

✅ `GET /api/v1/dashboards/energy` returns the dashboard with its panel.

### 3. Add a query variable (`$site`)

`options_config` is opaque (UI-owned); a `query` variable carries
`{datasourceId, sql}` whose first column is the option list.

```bash
post /api/v1/dashboards/energy/variables "$(python3 -c 'import json,os;print(json.dumps({
  "name":"site","label":"Site","kind":"query",
  "options_config":{"datasourceId":os.environ["DSID"],"sql":"SELECT DISTINCT site_id FROM telemetry_typed ORDER BY 1"},
  "multi":False,"include_all":True}))')"
```

✅ The option query returns `site-001 site-002 site-003`.

### 4. Confirm bind-safe injection (the headline correctness property)

Panel SQL references `$site`; values bind as `$N` args, never inlined. Pass
variable values on the query and prove an injection attempt is inert:

```bash
runq() { post /api/v1/datasources/$DSID/query "$1"; }
# real value → rows
runq '{"sql":"SELECT count(*) n FROM telemetry_typed WHERE site_id = $site","variables":[{"name":"site","values":["site-001"]}]}'
# injection attempt → 0 rows, NO error, table intact
runq '{"sql":"SELECT count(*) n FROM telemetry_typed WHERE site_id = $site","variables":[{"name":"site","values":["site-001'"'"'; DROP TABLE telemetry_typed; --"]}]}'
```

✅ Observed: real value → `n: 108`; injection value → `n: 0`, no error, table
still has all rows. The malicious string bound inert as one arg.

### 5. Mount the page on a nav node (with context)

`target` is `{"kind":"dashboard","dashboardId":"<uuid>"}`; `context.values` seeds
variables (read-only) for this mount.

```bash
post /api/v1/nav "$(python3 -c 'import json,os;print(json.dumps({
  "title":"Energy & Water","target":{"kind":"dashboard","dashboardId":os.environ["DASHID"]},
  "context":{"values":{"site":"site-001"}},"icon":"zap"}))')"
curl -s -b "$JAR" $BASE/api/v1/nav | python3 -c 'import sys,json;print([n["title"] for n in json.load(sys.stdin)])'
```

✅ The nav tree lists "Energy & Water" as a `dashboard` node carrying the seeded
`site` context.

---

## Acceptance criteria

- ✅ A panel queries the ingested data and returns a typed timeseries
  (timestamp/float), verified via the datasource query endpoint.
- ✅ Variable injection is **bind-safe** — a `'); DROP TABLE …` value binds inert
  (0 rows, no error, table intact). Verified live.
- ✅ The query variable's option query returns the live site list.
- ✅ Page is mounted on a nav node and appears in `GET /api/v1/nav`; the node
  carries the seeded `context.values`.
- ⬜ **UI render** (line chart actually drawing in the browser) — verify via the
  UI on :4790 / browser-testing; the API contract is proven, the visual is not.
- ⬜ Variable change bumps the query cache key (no stale frame) — a UI/runtime
  property, not exercised here.
- ⬜ Deleting the dashboard reverts dependent nav nodes to `group` — not yet run.

---

## Test data without a broker

You don't need the ingest path to test dashboards: `make seed-sim` populates
`sim_hvac` / `sim_energy` / `sim_door`. Build panels over those tables to test
the dashboard/variable/nav mechanics in isolation, then repeat against the live
ingested table once Flows ingest is green.

---

## Known issues / fixes

- ⚠️ **Tags write has no entity check** (`routes/tags/set.rs`) — a tag PUT on a
  bogus dashboard id succeeds. When testing the `context`/`tag` variable source,
  don't rely on tag writes validating the target. (Tracked authz gap.)
- _record fixes here_
