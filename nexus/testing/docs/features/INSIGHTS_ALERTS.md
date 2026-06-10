# Feature: Insights & Alerts

> Verified: nexus-rewrite tip on 2026-06-10 against the live stack (219 telemetry
> rows, elec 25.7–49.9 / water 1.2–3.6). **Status: end-to-end verified** — insights
> preview (all 3 error classes) and the full alert fire→resolve→silence lifecycle
> all pass. The scheduler is real and wired (see below).
> Reference: [WS-07_ALERTING](../../../docs/scope/nextgen/WS-07_ALERTING.md).

**What we're testing:** define an insight (post-query transform) over the ingested
meter data, and an alert rule that fires when a condition crosses a threshold —
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
- [ ] A saved insight referenced from a panel changes the rendered result. _(not
  re-run this sweep.)_

### Example insight (verified)

```jsonc
// method-style; df is the frame, final expression is the result
{
  "script": "df.zscore(\"value\").anomalies(\"value\", params.threshold)",
  "params": { "threshold": 2.0 }
}
```

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
