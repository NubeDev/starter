# Feature: Insights, Alerts & Detections/Findings

> Verified: nexus-rewrite tip on 2026-06-10 against the live stack (219 telemetry
> rows, elec 25.7–49.9 / water 1.2–3.6). **Status: end-to-end verified** — insights
> preview (all 3 error classes) and the full alert fire→resolve→silence lifecycle
> all pass. The scheduler is real and wired (see below).
> **Detections & Findings (WS-15) verified 2026-06-11** against the live stack via
> the `telemetry-pg` datasource (12 elec meters, peaks ~50): per-meter findings,
> dedup, auto-resolve, ack/resolve lifecycle, cascade-on-delete, AND the federated
> path all pass — see the new section below.
> Reference: [WS-07_ALERTING](../../../docs/scope/nextgen/WS-07_ALERTING.md),
> [WS-15_DETECTIONS_AND_FINDINGS](../../../docs/scope/nextgen/WS-15_DETECTIONS_AND_FINDINGS.md).

**What we're testing:** define an insight (post-query transform) over the ingested
meter data, an alert rule that fires when a condition crosses a threshold, and a
**detection** (an insight run on a schedule that emits per-target **findings**) —
all against the live MQTT→Postgres telemetry.

Architecture recap ([../reference/ARCHITECTURE.md §3](../reference/ARCHITECTURE.md)):
insights are sandboxed Rhai scripts over the result frame (bind `df` + `params`,
compose vectorized primitives, caps prevent row growth); alerts are
single/multi-condition rules with `interval_secs`/`for_secs`, firing to channels.

---

## Insights runbook (verified 2026-06-10)

> **Script API (confirmed against the engine, [nexus-insights/src/api.rs](../../../backend/crates/nexus-insights/src/api.rs)):**
> the bound frame is `df`; primitives are **methods that chain on the frame**, and
> the script's **final expression is the result**. The doc's old free-function form
> (`resample(df, "5m", "mean")`) is **wrong** — use `df.resample(time_col, every, [aggs])`
> and `df.anomalies(col, z)`. 22 primitives are registered (method-style).
> `params` is bound as a Rhai object, e.g. `params.threshold`.

1. [x] List available primitives: `GET /api/v1/insights/functions` → 22 functions
       with `name`/`signature`/`summary`/`category`/`example`.
2. [x] Author a script chaining primitives, e.g.
       `df.zscore("value").anomalies("value", params.threshold)`.
3. [x] `POST /api/v1/insights/preview` with `{script, rows, params}` → returns
       `ok:true` + a `QueryResponse`-shaped frame (HTTP 200), **without saving**.
       Script errors come back as `ok:false` + `{error:{kind,message}}`, still HTTP 200.
4. [ ] `POST /api/v1/insights` to save (compiles the script first, 400 on bad
       syntax); attach to a panel query via `InsightRef`. _(CRUD save not re-run this
       sweep; preview path fully exercised.)_

### Verified preview call

```bash
# rows = array of JSON objects; the frame columns are the object keys.
curl -s -b "$JAR" -X POST $BASE/api/v1/insights/preview \
  -H content-type:application/json -H "X-CSRF-Token: $csrf" -d '{
    "script": "df.zscore(\"value\").anomalies(\"value\", params.threshold)",
    "rows": [ {"time":"…","value":30.1}, … ],
    "params": { "threshold": 2.0 }
  }'
# → ok:true, result.columns = [time, value, value_anomaly, value_zscore], rows == input rows
```

### Insight acceptance

- ✅ Preview returns a transformed frame; 30 rows in → 30 rows out (no explosion).
- ✅ **All three error classes surface correctly (each triggered deliberately):**
  - syntax garbage (`this is ::: not rhai`) → `kind:"compile"`
  - unknown column (`df.zscore("nonexistent_col")`) → `kind:"runtime"`
  - infinite loop (`while true {…}`) → `kind:"limit"` ("Too many operations",
    trips `max_operations`; the 5s deadline is the other limit path — see
    [nexus-insights/src/limits.rs](../../../backend/crates/nexus-insights/src/limits.rs)).
- ✅ Determinism: same input frame + same params ⇒ byte-identical output rows
  (`df.zscore("value")` hashed identical across two calls).
- ✅ **A saved insight referenced from a panel changes the rendered result.**
  Verified 2026-06-10 — a panel query with `insight:{insight_id,params}` runs the
  stored script server-side after the SQL and returns its derived columns
  (`value_zscore`, `value_anomaly`, `value_roll_mean`). See the **insight-backed
  dashboard recipe** below.

### Example insight (verified)

```jsonc
// method-style; df is the frame, final expression is the result
{
  "script": "df.zscore(\"value\").anomalies(\"value\", params.threshold)",
  "params": { "threshold": 2.0 }
}
```

---

## Insight-backed dashboard recipe (VERIFIED, 2026-06-10)

A whole dashboard whose panels render **insight-derived** columns, not just raw
SQL output. The `insights-demo` dashboard (`/d/insights-demo`) was built this way
and all four panel queries verified live against `telemetry_typed`
(`elec`, `site-002`, 726 rows in a 09:00–13:00 window).

### How a panel attaches an insight

A panel persists `insight_id` + `insight_params` (own columns on `nexus_panels`,
**not** in the `layout` blob). The widget query hook
(`ui/src/features/widgets/useWidgetQuery.ts`) sends them as the optional
`insight:{insight_id,params}` on the **same** `/datasources/{id}/query` call that
carries `time_range` / `interval_secs` / `variables`. The server runs the panel's
SQL, then runs the stored insight script over the result frame, then serialises —
so the insight's new columns arrive as ordinary result columns the chart maps via
`layout.fields.series[]`. **Caps apply after the insight: it can shrink the frame,
never grow it.**

> ⚠️ **GOTCHA — whole-number params arrive as `i64`, breaking `f64`-typed
> primitives.** `anomalies(col, z: number)` is registered for `f64`. A param like
> `{"threshold": 2.0}` round-trips through jsonb/serde and reaches Rhai as **`2`
> (i64)**, so `anomalies("value", params.threshold)` fails with
> `Function not found: anomalies (Frame, String, i64)`. This bit the
> `elec-anomaly-zscore` insight live — curl with `2.0` worked, but the saved
> panel (whose `2.0` collapsed to an int) errored in the UI. **Fix in the script,
> not the caller:** coerce to float with arithmetic — `params.threshold * 1.0`
> (Rhai's `to_float()` is **not** registered for `i64` in this sandbox; use `* 1.0`
> or `+ 0.0`). The verified script is:
> ```rhai
> let z = params.threshold * 1.0;
> df.zscore("value").anomalies("value", z)
> ```
> Integer-typed primitives (`rolling_mean(col, window: int)`) are unaffected — they
> *want* `i64`, so `params.window` passes straight through.

> The insight's output column names are what you map in `layout.fields`. For the
> registered primitives: `zscore("value")` → `value_zscore`,
> `anomalies("value", z)` → `value_anomaly` (bool), `rolling_mean("value", n)` →
> `value_roll_mean` (note: `roll`, not `rolling`). Confirm names with a
> `/insights/preview` call before wiring a panel — a name typo = an empty series
> = "No data" with a working query (same trap as DASHBOARDS.md's `layout.fields`).

### Build (API)

```bash
# 1. save the insights (compiles the script; 400 on bad syntax)
ANOM=$(post /api/v1/insights '{"name":"elec-anomaly-zscore",
  "script":"df.zscore(\"value\").anomalies(\"value\", params.threshold)",
  "params_schema":{"type":"object","properties":{"threshold":{"type":"number","default":2.0}}}}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
ROLL=$(post /api/v1/insights '{"name":"rolling-mean-smooth",
  "script":"df.rolling_mean(\"value\", params.window)",
  "params_schema":{"type":"object","properties":{"window":{"type":"integer","default":5}}}}' \
  | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')

# 2. dashboard + a panel that references an insight by id (with per-panel params)
post /api/v1/dashboards '{"slug":"insights-demo","name":"Insights Demo"}'
post /api/v1/dashboards/insights-demo/panels "$(python3 -c 'import json,os;print(json.dumps({
  "title":"Z-score","datasource_id":os.environ["DSID"],
  "sql":"SELECT timestamp AS time, value FROM telemetry_typed WHERE $__timeFilter(timestamp) AND kind='elec' AND site_id = $site ORDER BY timestamp",
  "viz":"line","insight_id":os.environ["ANOM"],"insight_params":{"threshold":2.0},
  "layout":{"x":0,"y":0,"w":12,"h":7,"fields":{"x":"time","xKind":"time",
    "series":[{"value":"value_zscore","label":"Z-score"}]}}}))')"
```

The four panels built: **value + rolling-mean overlay** (`rolling-mean-smooth`,
two series `value`+`value_roll_mean`), **z-score line**, a **latest-z-score stat**
(reduce→last transform), and an **anomaly table** (time/value/zscore/anomaly).

### Verified (data layer, replaying each panel's exact widget request)

With `time_range` 09:00–13:00, `$site=site-002`, and the panel's `insight` ref:

| Panel | insight | params | result |
|-------|---------|--------|--------|
| value + rolling | rolling-mean-smooth | `window:5` | cols `value,value_roll_mean`, 34 buckets |
| z-score line | elec-anomaly-zscore | `threshold:2.0` | cols incl `value_zscore`, 726 rows, **25** anomalies |
| anomaly table | elec-anomaly-zscore | `threshold:1.5` | same cols, **108** anomalies |

✅ **Per-panel params are honoured** — the same stored insight flags 25 anomalies
at `threshold:2.0` and 108 at `1.5`, proving `insight_params` rides per-panel.
✅ **Refs round-trip** — `GET /api/v1/dashboards/insights-demo` returns each
panel's `insight_id` + `insight_params` + `layout.fields` intact.

To eyeball: `http://localhost:4790/d/insights-demo?from=now-6h&to=now` with
`$site=site-002` (widen the range if blank — data is "now"-ish while datapump runs).

- ⬜ **UI render** (the insight series actually drawing in the browser) — the data
  + persistence + request path are proven; the visual was not run this sweep (no
  Chrome DevTools MCP in the session). Verify via the link above or the Playwright
  harness pattern in CHARTS.md.

---

## Alerts runbook (verified 2026-06-10)

> **Scheduler IS wired.** `nexus_api::alerting::schedule::spawn()` is started at
> [main.rs:136](../../../backend/crates/nexus-api/src/main.rs); it ticks every
> **10 s**, claims due rules via `nexus_claim_due_alert_rules` (SECURITY DEFINER,
> `FOR UPDATE SKIP LOCKED`), and evaluates each under its tenant's RLS. Auto-fire works.
>
> **Key behaviours confirmed against [evaluate.rs](../../../backend/crates/nexus-api/src/alerting/evaluate.rs):**
> - A rule with `datasource_id: null` runs its query against the **dev pool** — the
>   same DB as `telemetry_raw`. No datasource setup needed for a local test.
> - The query must return one value; it's reduced by reducer `last` ("first row,
>   first column") for a legacy single-condition rule. Use `SELECT <agg>(value) AS value …`.
> - v1 channel `kind` is **`webhook`** (`config:{url}`); it POSTs the full
>   `Notification` JSON and requires a 2xx. (Also `slack`/`email` exist.)
> - The PUT `/alerts/rules/{id}` response has an **empty body** (200, not JSON) —
>   don't parse it.
> - The events list endpoint returns a **bare JSON array**, not `{events:[]}`.

A self-contained local recipe (no datapump needed — drive the threshold, not the data):

1. [x] Run a stub webhook sink that logs hits and returns 200 (e.g. a 15-line
       Python `http.server` on `:9099`).
2. [x] Create a channel: `POST /alerts/channels`
       `{"name":"…","kind":"webhook","config":{"url":"http://127.0.0.1:9099/hook"}}`.
3. [x] Create a rule that breaches: `POST /alerts/rules`
       `{"name":"…","query":"SELECT avg(value) AS value FROM telemetry_raw WHERE kind='elec'",
       "op":"gt","threshold":10.0,"for_secs":0,"interval_secs":10,"enabled":true,"channel_ids":[CH]}`.
4. [x] ✅ Within ~one tick the rule transitioned **ok→firing**; an event was recorded
       (`notified:true`) and the sink received the payload
       (`value 37.96 gt threshold 10`).
5. [x] ✅ Raise threshold to 100 (`PUT /alerts/rules/{id} {"threshold":100}`) →
       rule transitioned **firing→resolved**, a second webhook hit delivered.

### Alert acceptance

- ✅ Single-condition rule fires when the threshold is crossed (verified with
  `for_secs:0` — fires on first breach). `for_secs>0` routes ok→**pending**→firing
  via the state machine; the dwell gate is `(now - since) >= for_secs`
  ([evaluate.rs:77](../../../backend/crates/nexus-api/src/alerting/evaluate.rs)) —
  not separately timed this sweep but the pending path is in code.
- ✅ Resolve works: dropping the value under threshold transitions firing→resolved
  with a delivered notification.
- ✅ **A silence suppresses notification without stopping evaluation.** Verified:
  with an active silence covering the rule, lowering the threshold re-fired the
  rule (a `firing` **event was still recorded**) but with `silenced:true,
  notified:false`, and the webhook sink got **no new hit**. Matches the silence DTO
  contract ("suppresses notification, not evaluation").
- [ ] Multi-condition rule honors `combinator` + `no_data_policy` +
  `exec_error_policy` (no-data window + query error). _Not exercised this sweep_ —
  the policy/combinator code exists ([evaluate.rs:137](../../../backend/crates/nexus-api/src/alerting/evaluate.rs)
  `resolve_breaching`) but a deliberate no-data/error test is still TODO.
- ✅ **Scheduler confirmed running** (was the open ⚠️): `schedule::spawn` at
  [main.rs:136](../../../backend/crates/nexus-api/src/main.rs), 10 s tick. Rules
  auto-fired without any manual evaluation trigger.

---

## Deterministic alert scenarios

To test fire/resolve reliably, drive the data, not the clock: pick a datapump
seed + meter whose value sequence is known to cross the threshold at a known
publish index. Then the alert's fire is reproducible across runs.

---

## Known issues / fixes

- ✅ **Resolved:** the alert evaluator scheduler is wired and running (10 s tick,
  [main.rs:136](../../../backend/crates/nexus-api/src/main.rs)). The 2026-06-10
  sweep saw rules auto-fire, resolve, and respect silences with no manual trigger.
- ⚠️ **Doc-vs-reality gaps fixed this sweep:** insight scripts are method-style on
  `df` (not `resample(df,…)`); `PUT /alerts/rules/{id}` returns an empty body;
  `GET /alerts/events` returns a bare array.
- ⚠️ Still TODO: multi-condition rules (combinator + no_data/exec_error policies)
  and `for_secs>0` dwell timing were not driven this sweep — code paths exist but
  are unverified live. Insight **save** (`POST /insights`) CRUD also not re-run.
- _record fixes here_
