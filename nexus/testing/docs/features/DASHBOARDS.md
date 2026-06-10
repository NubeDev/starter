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

> **Panel/chart settings** (the editor's 7 tabs: viz type, field mapping, unit,
> decimals, thresholds, overrides, legend, axes, transforms) have their own
> reference: **[CHARTS.md](CHARTS.md)** — what each setting does, the config key
> it writes, the per-setting test dashboard (`/d/chart-settings`), and the
> persistence model (the whole display config rides in the `layout` blob; a bug
> that dropped everything but `fields` on save was fixed 2026-06-10).

---

## Time-range + variable-driven dashboard (VERIFIED recipe, 2026-06-10)

A full dashboard over the Zenoh telemetry (`telemetry_typed`), where the **page
time picker** and a **`$site` selector** drive every panel. The `energy`
dashboard was built this way and all four panel queries verified live.

### How it's driven (no UI code needed)

The panel query hook `ui/src/features/widgets/useWidgetQuery.ts` already sends,
with every panel's SQL:
- `time_range` — resolved from the page time picker (the `?from=now-1d/d&to=…`
  URL state via `features/time/`), so `$__timeFilter`/`$__timeGroup` get a window.
- `interval_secs` — auto-derived from the range, feeding `$__interval`.
- `variables` — the dashboard's resolved variable selections (`$site`, …).

So a panel whose SQL uses these macros is **automatically** wired to the time
picker + variable bar. Building the dashboard is pure data setup (panels + a
variable); the runtime mechanism exists.

### Time macros (server-side binder, all bound — never inlined)

| Macro | Expands to | Notes |
|-------|-----------|-------|
| `$__timeFilter(col)` | `col >= $from AND col < $to` | half-open, both bound |
| `$__timeGroup(col, '5m')` | dialect bucket expr | width literal **or** `$__interval` |
| `$__timeGroup(col, $__interval)` | bucket from the range | auto granularity |
| `$__timeFrom` / `$__timeTo` | one bound timestamp | |
| `$var` / `${var:csv}` / `$__sqlIn(var)` | bound `$N` arg(s) | values always inert |

**Gotchas (hit and fixed during verification):**
- The column arg to `$__timeFilter`/`$__timeGroup` must be a **bare identifier**
  (`timestamp`), **not** quoted (`"timestamp"`). The binder validates a bare
  name and re-quotes it itself; a quoted arg fails `invalid identifier`.
- `$__timeGroup` takes **2 args** (column, width). One arg → `expected 2 arguments`.

### The four panels (each time + `$site` scoped)

```sql
-- line: avg value over time by kind
SELECT $__timeGroup(timestamp, $__interval) AS t, kind, avg(value) AS value
FROM telemetry_typed
WHERE $__timeFilter(timestamp) AND site_id = $site
GROUP BY 1,2 ORDER BY 1

-- stat: avg value in window
SELECT avg(value) AS avg_value FROM telemetry_typed
WHERE $__timeFilter(timestamp) AND site_id = $site

-- bar: reading count by kind
SELECT kind, count(*) AS readings FROM telemetry_typed
WHERE $__timeFilter(timestamp) AND site_id = $site GROUP BY kind ORDER BY kind

-- table: recent rows
SELECT timestamp, site_id, kind, meter_id, value, unit FROM telemetry_typed
WHERE $__timeFilter(timestamp) AND site_id = $site ORDER BY timestamp DESC LIMIT 100
```

Build via the API (see step 1–3 below for datasource + variable): panels go to
`POST /api/v1/dashboards/{slug}/panels` with `{title, datasource_id, sql, viz,
layout}`; `viz` ∈ `line|bar|table|stat`. Update with `PATCH /api/v1/panels/{id}`.

> ⚠️ **GOTCHA that causes "No data" with a working query — the field mapping.**
> The backend has **no `fields` column**; the chart's column→role mapping rides
> **inside the opaque `layout` JSON** alongside the grid position
> (`ui/src/api/dashboards/panelAdapter.ts`). A panel created with only
> `layout:{x,y,w,h}` has an **empty series list** → the chart has nothing to plot
> → "No data", even though the query returns rows. You MUST stash the mapping:
>
> ```jsonc
> // layout for a line panel selecting columns `t` (time) and `value`
> { "x": 0, "y": 0, "w": 12, "h": 6,
>   "fields": { "x": "t", "xKind": "time",
>               "series": [{ "value": "value", "label": "Avg value" }] } }
> ```
> - `fields.x` = the x-axis column (omit for `stat`); `xKind: "time"` formats a
>   timestamp axis. `fields.series[].value` = a result column to draw.
> - **The `value`/`x` strings must match the SQL's output column names** (alias in
>   SQL so they're stable: `... avg(value) AS value`, `$__timeGroup(...) AS t`).
> - For a `table`, list every column you want shown as a `series` entry (with
>   `kind: time|number|text` for cell formatting).
> - The UI's panel editor writes this automatically; only **API-created** panels
>   need it set by hand. This is what bit the `energy` dashboard build (2026-06-10):
>   queries were fine, charts were blank until `layout.fields` was added.

The `$site` variable is a `query` kind over `SELECT DISTINCT site_id …` with
`current: ["site-001"]` so panels resolve on first load.

### Verified (data layer)

With `time_range` 09:00–11:00, `interval_secs: 60`, `$site=site-001`: line → 48
buckets, stat → 1, bar → 2 kinds, table → 100 rows; SQL-injection value for
`$site` binds inert (0 rows, table intact).

### NOT verified: visual render
The browser render (panels actually drawing) was **not** confirmed this session —
no Chrome DevTools MCP available. Eyeball it at
`http://localhost:4790/d/energy` with the time picker set to a window that
contains data (e.g. last 1–6h while datapump runs) and `$site` selected. If a
panel is blank, widen the time range first (the data is "now"-ish).

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
