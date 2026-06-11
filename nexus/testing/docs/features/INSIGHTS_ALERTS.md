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

## Detections & Findings runbook (WS-15, verified 2026-06-11)

> **The model (decided vocabulary):** an **Insight** run in **detection mode**
> produces **Findings**. A **Detection** is *a scheduled insight that emits
> findings* — the analytic sibling of an alert. Where an alert reduces a query to
> one scalar and compares it (one rule → one firing), a detection runs the same
> Rhai insight over the **whole result frame** and records **one finding per
> flagged row** (20 anomalous meters = 20 browsable, ackable findings).
> Notifications are out of scope: a detection *produces findings*, it does not
> page anyone. Reference:
> [WS-15](../../../docs/scope/nextgen/WS-15_DETECTIONS_AND_FINDINGS.md).
>
> **Scheduler IS wired**, mirroring the alert scheduler:
> `nexus_api::detecting::schedule::spawn()` at
> [main.rs](../../../backend/crates/nexus-api/src/main.rs) (`detection_scheduler`
> under the WS-16 task watchdog — confirmed in the boot log), 10 s tick, claims
> due detections via `nexus_claim_due_detections` (SECURITY DEFINER,
> `FOR UPDATE SKIP LOCKED`), runs each under its tenant's RLS.
>
> **Key behaviours confirmed against
> [run.rs](../../../backend/crates/nexus-api/src/detecting/run.rs):**
> - A detection = a saved **insight** + a **query** + a **datasource** + a
>   **schedule** + a column mapping (which output column is the flag, which
>   columns identify the target, which carries the value).
> - **`flag_column` empty ⇒ every returned row is a finding** (the `filter_gt`
>   "shrink the frame" pattern). A *named* column gates on its truthiness (the
>   `anomalies(col, z)` → `value_anomaly` pattern). Both verified live.
> - **Dedup:** a finding is keyed by `(detection_id, hash(target columns))` over
>   the **non-resolved** rows (partial unique index in
>   [2502_findings.sql](../../../backend/crates/nexus-store/migrations/nexus/2502_findings.sql)).
>   A target flagged N intervals running = **one** open finding (updated), not N.
> - **Auto-resolve:** a target that stops being flagged moves `open → resolved`
>   on the next run (mirrors the alert resolve transition).
> - **Lifecycle:** `open → acknowledged → resolved`, with `acked_by`/`acked_at`/
>   `note`; ack/manual-resolve via the API. Deleting a detection **cascades** its
>   findings.

A self-contained recipe over the live `telemetry-pg` datasource (12 elec meters,
peaks ~50; pick a threshold that flags a few):

```bash
# the insight (the rule): keep only rows over params.limit → every returned row
# a finding. Author once; many detections can reference it with different params.
IID=$(post /api/v1/insights '{"name":"high-usage",
  "script":"df.filter_gt(\"value\", params.limit * 1.0)"}' | jq -r .id)

# the detection: peak elec per meter, run against the telemetry-pg datasource,
# flag meters whose peak exceeds 48, identify by meter, carry value.
DSID=$(curl -s -b "$JAR" $BASE/api/v1/datasources | jq -r '.[]|select(.name=="telemetry-pg").id')
DID=$(post /api/v1/detections "$(jq -cn --arg i "$IID" --arg d "$DSID" '{
  name:"high-elec", insight_id:$i, datasource_id:$d,
  sql:"SELECT meter_id, max(value) value FROM telemetry_typed WHERE kind='elec' GROUP BY meter_id",
  params:{limit:48.0}, flag_column:"", target_columns:["meter_id"],
  value_column:"value", interval_secs:600}')" | jq -r .id)

post /api/v1/detections/$DID/run          # run now, off-schedule (don't wait)
curl -s -b "$JAR" "$BASE/api/v1/findings?detection_id=$DID" | jq   # one per offending meter
curl -s -b "$JAR" "$BASE/api/v1/detections/$DID/stats" | jq        # run stats (see below)
```

### Verified live (against `telemetry-pg`, 2026-06-11)

1. [x] ✅ **Per-meter findings.** `limit:48` over `telemetry_typed` produced **one
       finding** for `site-002-elec-004` (50.55), carrying
       `{target:{meter_id,site_id}, value, context}`. Lowering to `40` → **5**
       findings (the meters >40); raising to `49` → back to **1**.
2. [x] ✅ **Dedup.** Running the same detection twice with the same data kept
       **one** open finding per meter, not two — the second run updates in place.
3. [x] ✅ **Auto-resolve.** After lowering to 40 (5 open), raising to 49
       auto-resolved **4** and left **1** open — the meters that dropped under the
       new threshold moved to `resolved` on the next run.
4. [x] ✅ **Ack → resolve lifecycle.** `POST /findings/{id}/ack` →
       `acknowledged` with `acked_by` = the admin's UUID + `acked_at` + note;
       `POST /findings/{id}/resolve` → `resolved`.
5. [x] ✅ **Cascade on delete.** `DELETE /detections/{id}` dropped its findings
       (open count 4 → 0).
6. [x] ✅ **Findings nav + browse.** `Findings` is a seeded `route` nav node
       (Automation group); `GET /findings?status=&detection_id=` filters the feed.

### Run stats — `GET /api/v1/detections/{id}/stats`

A glanceable summary for the list/editor:
`{ next_eval_at, last_finding_at, open, acknowledged, resolved, total }`.
Verified: **before** any run `open:0, last_finding_at:null`; **after** a run
`open:3, last_finding_at:<stamped>`. One cheap aggregate query (the
`(tenant_id, detection_id, status)` index covers it) — no N+1 over findings. The
Detections list renders it per row ("3 open · 3 findings · last spark 2m ago ·
next run in 9m").

### Detection editor (UI: Findings → Detections tab)

The detection editor is the insight picker + a **datasource picker** + query +
flag/target/value column mapping + schedule + optional federated sources. Each
detection row has **Edit** (✏️), **Run now** (▶), and **Delete** (🗑). Verified
live: create, edit (incl. **changing the datasource**, **clearing it to the dev
pool**, and a name-only edit that leaves the datasource untouched), run, delete.

> ⚠️ **`datasource_id` clear uses a `clear_datasource` flag, not a JSON `null`.**
> The update DTO can't tell an explicit `null` from "field absent" on the wire
> (serde collapses both) — so clearing a datasource needs
> `PUT {"clear_datasource":true}`, and setting one needs
> `PUT {"datasource_id":"<uuid>"}`. The UI's "Dev pool" option sends the flag.
> This is the **same trap** the panel editor's `clear_insight` flag avoids
> ([panel/update.rs](../../../backend/crates/nexus-spi/src/dto/panel/update.rs)) —
> it bit detections too in live testing (an explicit `null` was a no-op) and is
> fixed the same way.

### Federated detections (cross-datasource / file joins)

A detection reaches **everything a panel query can** — including RW-05 federation.
Provide `sources` (an alias→datasource array) and the runner dispatches through
the federation engine instead of the single-datasource push-down path. Verified
live: a detection with
`sources:[{"alias":"tel","datasource":"<telemetry-pg id>","table":"telemetry_typed"}]`
and `sql:"SELECT meter_id, max(value) value FROM ds_tel WHERE kind='elec' GROUP BY meter_id"`
ran through the engine and produced **3** findings (meters whose peak >48).

> ⚠️ **GOTCHA — federated SQL is DataFusion, not Postgres.** A federated query
> runs in the engine, so Postgres-only syntax behaves differently. Live: a
> `SELECT DISTINCT ON (meter_id) …` returned a **different "latest" row per
> meter** than the push-down path (DataFusion's `DISTINCT ON` doesn't honour the
> `ORDER BY timestamp DESC` tiebreak the way Postgres does), so the same logic
> found 0 findings federated vs 1 push-down. **Use portable SQL** (`max(value) …
> GROUP BY`) for federated detections — it gave identical, correct results on
> both paths. This is a federation-wide trait (panels hit it too), not a
> detection bug.

> 💡 **Whole-number param gotcha applies here too.** A float-typed primitive
> (`anomalies(col, z:f64)`) breaks when a param like `2.0` round-trips as `i64` —
> coerce in-script with `params.z * 1.0`. Same fix as the insight section's
> GOTCHA; the detection editor's params field notes it inline.

### Detection acceptance

- ✅ One finding per offending target; `{target, value, context}` populated.
- ✅ Dedup: N consecutive flags = one open finding.
- ✅ Auto-resolve when a target stops flagging; manual ack/resolve via API.
- ✅ `flag_column` empty = every row; named column = truthiness gate.
- ✅ Runs against a tenant datasource **or** the dev pool (`datasource_id:null`);
  datasource is set at create and editable (set/change/clear) via PUT.
- ✅ Federated `sources` dispatch through the engine (cross-datasource/file).
- ✅ Run stats endpoint; cascade-on-delete; findings nav + filtered browse.
- [ ] **Scheduler auto-fire on the 10 s tick** not separately timed this sweep
  (used `/run` for determinism) — but `detection_scheduler` is armed under the
  task watchdog (boot log) and is a near-copy of the verified alert scheduler.
- [ ] **`for_secs` dwell** on detections is wired in the schema (default 0) but
  the v1 runner treats it as point-in-time; debounce path is TODO.
- [ ] **Findings dashboard panel kind** (count/list/trend-markers) — WS-15 §6,
  deferred (a canvas/widgets change, not built this sweep).

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
- ✅ **Detections & Findings (WS-15) verified live 2026-06-11** — see the runbook
  above. Per-meter findings, dedup, auto-resolve, ack/resolve, cascade-on-delete,
  run stats, and the federated path all pass against `telemetry-pg`.
- ⚠️ **Detection gotchas found + fixed this sweep:**
  - The detection editor shipped with **no datasource picker** and
    `datasource_id` was **not in the update path** — a detection's datasource was
    set-once-via-API and immutable. Fixed: added the picker (create + edit) and a
    `clear_datasource` flag so set/change/clear all work.
  - `datasource_id` clear can't use JSON `null` (serde can't tell `null` from
    absent) — uses a `clear_datasource` bool, same as panel `clear_insight`.
  - Federated SQL runs in DataFusion, not Postgres: `DISTINCT ON` semantics
    differ — use portable `GROUP BY` for federated detections.
- _record fixes here_
