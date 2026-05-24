# 2026-05-24 — Goal 6 landed end-to-end + durable cron scheduler upstream

Closing session note for branch `codeless/rubix-goal-6-weekly-report`.
Five of six SCOPE goals now light up end-to-end through the real
agent loop; Goal 1 — Dashboards remains a stub-output verb pending
SDUI page-store + `dashboard.*` verbs (see
[THIN-SLICE.md §"Goals lit up beyond the thin slice"](../scope/THIN-SLICE.md)).

The job was framed as "wire Goal 6," but the first stage's review
caught that the bundled `weekly-report.yaml` declared a 5-field
cron that the in-tree scheduler stub rejected. Rather than rewrite
the YAML to fit the limited parser, the work pivoted upstream: a
new `starter-cron` crate, a durable `scheduled_flows` table, and a
real `FlowAsService` body landed first; Goal 6 then consumed them.
GAPS row 16 (`FlowAsService` named) is therefore addressed in the
same branch — see
[`docs/design/scheduling/README.md`](../design/scheduling/README.md).

## Phases — what landed where

### Phase A — upstream cron + schedule table

- **A.1 — `starter-cron` crate.** New crate at
  [`crates/starter-cron/`](../../../crates/starter-cron/) accepts
  5/6/7-field cron expressions, normalises overflow (e.g. `dow=7`),
  exposes `parse` + `Schedule::next_fire`. Replaces the previous
  5-field-only parser that rejected `weekly-report.yaml`. Resolves
  SCOPE OQ-1 in favour of a standalone crate (rationale: a future
  `starter-cron-cli` wants the parser without pulling in flow
  types). Commit `5dac211`-era.
- **A.2 — `scheduled_flows` PG migration.**
  [`crates/starter-store-postgres/migrations/scheduled_flows/0001_init.sql`](../../../crates/starter-store-postgres/migrations/scheduled_flows/0001_init.sql)
  — ULID PK, `UNIQUE (tenant_id, flow_id)`, `pg_notify` trigger on
  the `starter_scheduled_flows` channel for insert and for changes
  to `next_run_at` / `enabled`. Integration test asserts the
  NOTIFY actually fires under testcontainers PG.
- **A.3 — `starter-flow-nodes::trigger_schedule` body.** The 23-line
  stub became a real `NodeBehavior` that reads `cron_expr` from the
  node config and exposes it on its output slot. The node is a
  passive entry node; firing comes from Phase B's tick. Commit
  `d08a587` → `3f8c109`.

### Phase B — `FlowAsService` register + tick

- **B.1 — Scaffold + Clock.**
  [`crates/starter-flow-surfaces/src/service.rs`](../../../crates/starter-flow-surfaces/src/service.rs)
  introduces `FlowAsService` holding `Pool<Postgres>` +
  `Arc<FlowRegistry>` + `Arc<dyn FlowRunner>` + `Arc<dyn Clock>`.
  [`clock.rs`](../../../crates/starter-flow-surfaces/src/clock.rs)
  carries `SystemClock` (production) and `TestClock` (deterministic
  in tests). `register_schedule` / `unregister_schedule` write to
  `scheduled_flows` and emit `pg_notify`. Commits `e294edb` →
  `68b60c6`.
- **B.2 — Tick + start.** `tick()` claims due rows under
  `SELECT FOR UPDATE SKIP LOCKED LIMIT 32`, dispatches each via
  `FlowRunner::run`, writes `last_run_*`, and recomputes
  `next_run_at` via `starter_cron::next_fire`. `start(self) ->
  JoinHandle<()>` spawns the tick loop on `tick_interval_seconds`.
  Integration test
  [`scheduled_flows_tick_test.rs`](../../../crates/starter-flow-surfaces/tests/scheduled_flows_tick_test.rs)
  uses `TestClock` to advance 2 minutes and asserts two fires.
  Commits `d94f147` → `0783882`.

### Phase C — rubix analytics verbs

- **C.1 — `analytics.query` + six named CH templates.** Implements
  [`rubix/crates/rubix-tools/src/analytics/query.rs`](../../../crates/rubix-tools/src/analytics/query.rs)
  as `AnalyticsQueryTool` taking `{ name, params }`, looking up a
  template via `include_dir!(./templates)`, binding params using
  ClickHouse `{name:Type}` syntax, running against the threaded
  `ChClient`. Six templates ship: `disk_history_weekly`,
  `alert_count_weekly`, `flow_run_summary_weekly`,
  `user_activity_weekly`, `clickhouse_writes_weekly`,
  `undo_count_weekly`. Three MessageKeys
  (`query.ran` / `query.unknown_template` / `query.bind_error`) in
  EN + ES same commit. Sibling test asserts each template runs
  under testcontainers CH with synthetic data. Commits `bfc1e37`
  → `cbf2183`.
- **C.2 — `analytics.report` verb.**
  [`rubix-tools/src/analytics/report.rs`](../../../crates/rubix-tools/src/analytics/report.rs)
  implements `AnalyticsReportTool` taking `{ template, queries[],
  format }`, runs each named query, hands rows to `starter-export`
  per format (html / csv / json; pdf returns
  `format_unsupported`), writes bytes through `starter-blob-fs::write`,
  and returns `{ blob_id, url, byte_count, format }`. Implements
  `Reversible` — `revert = starter-blob-fs::delete`. Three new
  MessageKeys (`report.rendered` / `report.empty` /
  `report.format_unsupported`) in EN + ES. Integration test
  asserts HTML report bytes contain expected per-day rows. Commit
  `ea07623`.

### Phase D — wire Goal 6 + integration test

- **D.1 — `weekly-report.yaml` rewrite + delete `WeeklyReportStub`.**
  Removed `rubix-tools/src/analytics/weekly_report.rs`; updated
  `analytics/mod.rs` to drop the stub export and add `query` +
  `report`. Rewrote
  [`rubix-flows/flows/weekly-report.yaml`](../../../crates/rubix-flows/flows/weekly-report.yaml):
  `trigger: explicit` → `trigger: schedule` with
  `cron_expr: 0 8 * * 1` (Monday 08:00 UTC); `allowed_tools` becomes
  `[analytics.query, analytics.report, rubix.alert.send,
   rubix.undo.last]`. Skill SKILL.md rewritten present-tense.
  Commit `8fd1fb6`.
- **D.2 — Wire `FlowAsService` at boot.**
  [`rubix-agent::main`](../../../crates/rubix-agent/src/main.rs)
  constructs `FlowAsService` from the existing PG pool +
  `FlowRegistry` + `FlowRunner` + a fresh `SystemClock`. First
  boot seeds `scheduled_flows` from every bundled YAML carrying
  `trigger: schedule` (just `weekly-report` initially); then calls
  `FlowAsService::start` to spawn the tick task. `AgentConfig`
  grew a `[scheduler]` section in
  [`boot/config.rs`](../../../crates/rubix-agent/src/boot/config.rs)
  (`enabled` default true, `tick_interval_seconds` default 60).
  Commit `786e7e7`.
- **D.3 — End-to-end test + reports design doc.**
  [`rubix-agent/tests/goal_6_weekly_report_test.rs`](../../../crates/rubix-agent/tests/goal_6_weekly_report_test.rs)
  boots against testcontainers PG + CH + tempdir blob backend,
  pre-populates `system_disk_history` with 7 days of synthetic
  data, advances `FlowAsService`'s `TestClock` by 7 days to trigger
  one fire, asserts the flow runs to completion, the agent reply
  is non-empty, an HTML report blob lands, and the bytes contain
  the expected per-day rows. Then calls `rubix.undo.last` and
  asserts the blob is gone.
  [`docs/design/reports/README.md`](../design/reports/README.md)
  rewritten present-tense. Commit `3742674`.

## Operator-runnable manual flow

The same path is reachable without the test harness. Boot
rubix-agent against a fresh PG + CH (e.g. `mani run run`); then
from the host:

```bash
# 1. Confirm the schedule seeded.
psql -d rubix -c "
  SELECT flow_id, cron_expr, enabled, next_run_at
  FROM   scheduled_flows
  WHERE  flow_id = 'com.rubix.weekly-report';
"
# → one row, cron_expr = '0 8 * * 1', next_run_at = next Monday 08:00 UTC.

# 2. Force-fire by setting next_run_at into the past, then wait one tick.
psql -d rubix -c "
  UPDATE scheduled_flows
  SET    next_run_at = now() - interval '1 minute'
  WHERE  flow_id = 'com.rubix.weekly-report';
"
sleep 65  # default tick_interval_seconds = 60

# 3. Inspect the fire result and pick up the blob id from the agent reply.
psql -d rubix -c "
  SELECT last_run_status, last_run_at, last_run_message
  FROM   scheduled_flows
  WHERE  flow_id = 'com.rubix.weekly-report';
"
# → last_run_status = 'success'; last_run_message has the blob id.

# 4. Download the rendered HTML report by its blob id.
BLOB=<id from step 3>
curl -b cookies.txt http://127.0.0.1:8088/api/v1/blob/$BLOB > weekly.html
# → ~5–20 KB of HTML carrying one section per template
#   (disk_history_weekly first, then the other five).

# 5. Undo the report (deletes the blob).
curl -b cookies.txt -X POST \
  http://127.0.0.1:8088/api/v1/tools/rubix.undo.last
curl -b cookies.txt -I http://127.0.0.1:8088/api/v1/blob/$BLOB
# → 404
```

This is the boot → fire → blob → undo loop the review gate
required.

## Evidence summary

- `tools/call`: `analytics.query` + `analytics.report` (both via
  the bundled `weekly-report` flow; tick-driven, not explicit).
- Undo: `rubix.undo.last` deletes the report blob through
  `starter-blob-fs::delete`.
- Tests:
  [`rubix-agent/tests/goal_6_weekly_report_test.rs`](../../../crates/rubix-agent/tests/goal_6_weekly_report_test.rs)
  — full boot → fire → blob → undo, plus the Phase A/B/C sibling
  tests under each crate.
- Boot log: `i18n_keys` grew by 6 (3 for `query.*`, 3 for
  `report.*`) in both EN and ES catalogues.
- Cross-process: `pg_notify('starter_scheduled_flows', …)` fires
  on every `register_schedule`; a future sidecar can listen
  without polling. Verified by the Phase A.2 migration test.

## Open questions

- **OQ-1 — standalone `starter-cron` vs folded into
  `starter-flow-spi`.** Resolved in favour of standalone — see
  Phase A.1 above and the
  [starter-changes ledger entry](../design/starter-changes/README.md).
- **OQ-2 — backfill on missed windows.** The scheduler does not
  catch up; if `next_run_at` is six fires in the past, one fire
  runs and `next_run_at` advances to the next future boundary.
  Operators wanting backfill use a one-shot `trigger: explicit`
  invocation. Documented in
  [`docs/design/scheduling/README.md`](../design/scheduling/README.md)
  §"What this does not do."

## After this branch

- Goal 1 (Dashboards) is the last remaining stub. Unblocks when
  the SDUI page store and `dashboard.{create,update,list,page.set,duplicate}`
  verbs land — out of scope for this branch.
- The cron primitive is now available for every future scheduled
  flow; the next bundled flow that wants a cron trigger just adds
  `trigger: schedule` + `cron_expr` to its YAML and the boot
  seeder picks it up.
