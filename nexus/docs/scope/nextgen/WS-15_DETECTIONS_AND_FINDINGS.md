# WS-15 — Detections & Findings: a SkySpark-style analytic rule engine

> **Status:** Not started · **Wave:** 2 (builds on insights + the alert scheduler) · **Owner:** _unassigned_
> **Depends on:** RW-06 insights (the Rhai engine + the panel `insight_id` link, shipped) and the
> WS-07 alert scheduler pattern (reuse, don't fork). No hard new infra.
> **Migration:** block `25xx` (e.g. `2501_detections.sql`, `2502_findings.sql`).
> **Read first:** `WS-07_ALERTING.md` (scheduler + event-log pattern to mirror),
> `testing/docs/features/INSIGHTS_ALERTS.md` (the insight engine, verified), GAP_ANALYSIS §2.7.
> **Verified:** code claims below grepped on 2026-06-11 — re-grep file:line before building.

## Goal

Today an **alert** is a tripwire: one SQL query → reduce to one scalar → `op`/`threshold` → fire a
notification. That answers "is *this number* over *that line* right now" and nothing else. It cannot
say *which* of 200 meters is anomalous, cannot flag *every* interval above baseline, and produces no
durable record you can browse, trend, or acknowledge.

What an energy/analytics platform actually needs is the **SkySpark/Axon model**: you write a *rule*
(rich logic, not one scalar), it runs over history **on a schedule**, and each match becomes a
persistent **finding** — a "spark" — with full context (which site, which meter, when, what value,
why). You browse findings, acknowledge them, trend them, surface them on dashboards.

This WS adds that. The **rule logic is an insight** (the Rhai engine from RW-06 — already built and
verified), run in **detection mode**: instead of rendering derived columns on a panel, the same script
*flags rows*, and each flagged row is recorded as a finding. The scheduler is a near-copy of the
proven alert runner. Notifications stay out of scope here — alerting remains the (separate, basic)
notify path; a detection's job is to **produce findings**, not to page anyone. (Wiring a finding to a
notify channel is an explicit follow-up, not this WS.)

**Vocabulary (decided):** an **Insight** run in **detection mode** produces **Findings**. UI surfaces:
"Insights" (unchanged, the authoring/transform surface) and "Findings" (the new browse/trend/ack
surface). "Detection" is the noun for *a scheduled insight that emits findings*.

## Current state (evidence) — what to reuse, not rebuild

- ✅ **Insight engine** — `nexus_insights::run_insight_rows(script, rows, params) -> Vec<Value>`
  (`crates/nexus-insights/src/run.rs:27`). Sandboxed Rhai, row-count never grows, all 3 error classes
  verified. Primitives like `anomalies("value", z)` already emit a per-row boolean column
  (`value_anomaly`). **This is the finding generator — it exists.**
- ✅ **Insight apply seam** — `crates/nexus-api/src/insights/apply.rs:18` runs a stored insight over a
  query result frame, resolving the script by `insight_id` under the tenant. The detection runner calls
  the *same* path.
- ✅ **Stored insight** — `InsightRecord { id, name, script, params_schema }`
  (`crates/nexus-store/src/insight/record.rs`), tenant-scoped via RLS (`2101_insights.sql`).
- ✅ **Scheduler pattern to mirror** — `crates/nexus-api/src/alerting/schedule.rs`: a 10s `TICK`
  (`:18`), `claim_due` over `FOR UPDATE SKIP LOCKED` so it's replica-safe (`:14`,`:40`), `run_once`
  for deterministic tests (`:39`), errors swallowed per-rule so one bad rule never stalls the loop.
- ✅ **Event-log pattern to mirror** — `nexus_alert_events` (`0004_alerting.sql`): append-only,
  tenant-scoped, `{rule_id, at, value, detail, …}`, with a `GET /alerts/events` browse endpoint.
  Findings are this, generalised (per-row, with a target + lifecycle).
- ⚠️ **Alerts stay basic on purpose.** WS-07 already covers making the *tripwire* better
  (multi-condition, channels). WS-15 does **not** touch alerting; it's the orthogonal "analytics →
  findings" axis. A finding may *later* trigger an alert action — out of scope here.

## The model (three layers, today conflated into "alert")

```
Detection (a scheduled rule)              Finding (a persistent spark)
┌────────────────────────────┐           ┌──────────────────────────────┐
│ insight_id  (the Rhai rule) │   run     │ detection_id                 │
│ datasource_id + SQL         │  ──────▶  │ site / meter / target keys   │
│ params (thresholds, window) │  each     │ at (event time)              │
│ flag_column ("value_anomaly")│  flagged │ value + context (jsonb)      │
│ target_columns (site,meter) │   row     │ status: open|ack|resolved    │
│ schedule (interval_secs)    │  ──────▶  │ dedup_key (one open per key) │
└────────────────────────────┘           └──────────────────────────────┘
        ▲ reuses RW-06 insight engine             ▲ generalises nexus_alert_events
```

- A **detection** is a saved insight + a query + a schedule + which output column means "flagged" and
  which columns identify the *target* (site_id, meter_id, …).
- The runner: claim due detections → run the SQL (guarded path) → `run_insight_rows` the insight over
  the result → for each row where `flag_column == true`, upsert a **finding** keyed by
  `(detection_id, target keys, bucketed time)`.
- A finding is **per-row / per-meter**, not one-per-rule — the core difference from an alert. Twenty
  anomalous meters in one run = twenty findings, each browsable and ackable.

## Scope

1. **`detections` store + DTO** (`2501_detections.sql`) — `{ id, tenant_id, name, insight_id (FK,
   ON DELETE SET NULL? or RESTRICT — decide), datasource_id, sql, params jsonb, flag_column text,
   target_columns text[], value_column text, time_column text, interval_secs, for_secs?, enabled,
   next_eval_at }`. RLS + `nexus_runtime` grants exactly like `nexus_insights`. A detection *references*
   an insight — it does not duplicate the script (one insight, many detections with different params).

2. **`findings` store + DTO** (`2502_findings.sql`) — generalises `nexus_alert_events`:
   `{ id, tenant_id, detection_id (FK ON DELETE CASCADE), at, target jsonb (the identifying column
   values), value double precision, context jsonb (the flagged row's other derived columns, e.g.
   zscore), status text (open|acknowledged|resolved), acked_by, acked_at, note, dedup_key text,
   created_at, updated_at }`. Index `(tenant_id, detection_id, status)` and `(tenant_id, status, at)`
   for the browse/trend queries. **Dedup:** unique `(detection_id, dedup_key)` among non-resolved rows
   so a meter that's anomalous for 6 straight intervals is **one open finding**, not six — it updates
   `value`/`at` instead. Auto-resolve a finding whose target stops being flagged (mirrors alert
   resolve).

3. **Detection runner** (`nexus-api/src/detecting/` — sibling of `alerting/`) — a near-copy of the
   alert scheduler: `TICK`/`claim_due`/`FOR UPDATE SKIP LOCKED`/`run_once`, spawned in `main.rs`
   alongside `alerting::schedule::spawn`. Per detection: resolve the datasource pool (reuse
   `evaluate.rs`'s `resolve_pool`), run the SQL under the **same query guards**, call
   `nexus_insights::run_insight_rows` with the detection's params, then iterate rows → upsert/resolve
   findings. Errors logged per-detection, never stall the loop. **Caps apply after the insight** (it
   shrinks, never grows the frame), so a runaway detection can't flood the findings table beyond the
   query cap.

4. **Findings API** — `GET /api/v1/findings` (filter by detection/status/site/meter/time range),
   `GET /api/v1/findings/{id}`, `POST /api/v1/findings/{id}/ack`, `POST /.../resolve` (manual),
   `GET /api/v1/detections` + CRUD. Findings list mirrors the `GET /alerts/events` shape + paging.

5. **Findings UI ("Findings" nav item)** — a list/table: rule, target (site/meter), when, value, why
   (context), status; filter + sort; per-row **Acknowledge / Resolve / Open in Explore** (deep-link to
   the detection's query + insight so the analyst sees the data behind the spark). Open-findings count
   badge on the nav (reuse the pattern). A detection editor that's basically the insight picker (from
   WS panel work) + datasource/SQL + flag/target column mapping + schedule.

6. **Findings on dashboards** — a new panel/widget **kind = `findings`**: render open findings for a
   detection (count stat, list, or **markers on a trend** overlaid on a time-series panel). Findings
   become first-class dashboard content. Couples lightly with the panel `insight_id` work already
   shipped — same "panel references a saved analytic object by id" pattern.

7. **Acknowledge / resolve lifecycle** — `open → acknowledged → resolved`, with `acked_by`/`acked_at`/
   `note`; auto-resolve on condition-clear; manual resolve via API/UI. A findings list is a *workflow*,
   not just a log.

## Design notes

- **Reuse, don't fork.** The runner is the alert scheduler with the body swapped (insight-over-frame
  instead of reduce+compare); the findings store is `nexus_alert_events` generalised. Lifting the
  shared scheduler scaffolding (`claim_due` + tick loop) into one helper both runners call is a nice-
  to-have, not required for v1 — a parallel module is fine and lower-risk.
- **The rule logic is the insight — no new DSL.** A detection adds only *which column is the flag* and
  *which columns are the target*; everything expressive lives in the Rhai script the user already
  authors and previews in the Insights Workbench. "Find high usage" = an insight like
  `df.filter_gt("value", params.limit)` (every returned row is over the limit → every row a finding),
  or `df.zscore("value").anomalies("value", params.z)` with `flag_column = "value_anomaly"`.
- **Per-row findings need a target + dedup key.** Without `target_columns` + `dedup_key`, you either
  get one finding per rule (useless, that's an alert) or a new finding every interval (noise). The
  dedup key = hash of the target column values (+ optional time bucket). This is the single most
  important design decision in the WS — get it wrong and the findings table is either too coarse or
  unusable noise.
- **Whole-number param gotcha (already documented).** Float-typed primitives (`anomalies(col, z:f64)`)
  break when a param like `2.0` round-trips as `i64`; coerce in-script with `params.z * 1.0`. See
  `testing/docs/features/INSIGHTS_ALERTS.md` "GOTCHA". Detection params hit the same path — document it
  on the detection editor.
- **Caps + safety are inherited.** Query runs under the existing read-only/timeout/row-cap guards; the
  insight runs under its sandbox (ops/depth/wall-clock). A detection cannot do more than a panel query
  + a preview can already do — it just does it on a schedule and writes findings.
- **Insight deletion vs detection.** Decide FK posture: `ON DELETE RESTRICT` (can't delete an insight a
  detection uses — safer) vs `SET NULL` (detection survives, disabled). Lean RESTRICT for detections
  (a detection with no rule is meaningless), unlike panels where SET NULL was right (the panel still
  renders raw SQL).

## Non-goals (this WS)

- **Notifications.** A finding does not page anyone in v1. Wiring a finding → alert channel
  (webhook/slack/email) is a clean follow-up once findings exist; the alert `Notifier` machinery is
  reusable then. Explicitly out of scope to keep this WS to "produce + browse findings".
- **Rewriting alerts.** WS-07 owns making the tripwire better. WS-15 is the orthogonal analytics axis.
  They may converge later (an alert becomes "a detection whose action is notify"), but not here.
- **Backfill / historical re-scan.** v1 runs forward on a schedule. Re-running a detection over a past
  window to populate findings retroactively is a follow-up.
- **Cross-tenant / fleet rollups.** Findings are tenant-scoped like everything else.

## Acceptance

- ✅ A detection (insight + SQL + schedule) created via API runs on its interval without manual
  triggering (scheduler wired in `main.rs`, verified the same way the alert scheduler was).
- ✅ Running "find high usage" over `telemetry_typed` produces **one finding per offending meter**,
  each carrying `{ site, meter, at, value, context }` — verified live against seeded energy data.
- ✅ Dedup: a meter flagged for N consecutive intervals = **one open finding** (updated), not N.
- ✅ Auto-resolve: when the meter stops being flagged, its finding moves to `resolved`.
- ✅ `GET /findings` filters by detection/status/site/time; `POST /findings/{id}/ack` transitions
  `open → acknowledged` with `acked_by`.
- ✅ A `findings` dashboard panel renders open findings for a detection (count + list).
- ✅ Determinism: same input frame + same params ⇒ same findings (reuses the insight engine's verified
  determinism).
- ✅ Deleting a detection cascades its findings; the FK posture on insight-delete behaves as decided.

## Open questions (resolve before building)

1. **Dedup key granularity** — target-only (one open finding per meter until resolved) vs
   target + time-bucket (one per meter per day)? Affects noise vs history fidelity. *Lean: target-only
   with auto-resolve; the finding's history is the audit/ack trail, time-series lives in trends.*
2. **`for_secs` dwell on detections?** Should a row have to stay flagged for N seconds before it's a
   finding (debounce), like alert `for_secs`? *Lean: optional, default 0 — most analytic findings are
   point-in-time.*
3. **Insight-delete FK** — RESTRICT vs SET NULL (see design notes). *Lean: RESTRICT.*
4. **Findings retention** — cap/rolling-delete old resolved findings (like changelog retention,
   `1603_changelog_retention.sql`)? *Lean: yes, a retention job, but follow-up.*
