# REPORTS

Analytics report rendering for rubix. The `rubix.analytics.report`
verb stitches a list of named [`rubix.analytics.query`](../../../crates/rubix-tools/src/analytics/templates/)
templates into one rendered artifact (`html` / `csv` / `json`),
pushes the bytes through [`starter-export`](../../../../crates/starter-export/),
and persists the result through any `starter_spi::blob::BlobStore`
(production wires this to [`starter-blob-fs`](../../../../crates/starter-blob-fs/)).
The mint is undo-aware: every successful render records an
`Op::Create` row through [`starter-undo`](../../../../crates/starter-undo/),
and `rubix.undo.last` reverses the create by deleting the blob.

## Pipeline

```
       ┌──────────────────────────────┐
       │  ai-agent (skill-driven)     │
       │  com.rubix.analytics-reporter│
       └─────────────┬────────────────┘
                     │ tool_use
       ┌─────────────▼───────────────┐
       │  rubix.analytics.query      │  ──►  ChClient  ──►  ClickHouse
       │  (named SQL templates)      │       (one template per file)
       └─────────────┬───────────────┘
                     │ rows[]
       ┌─────────────▼───────────────┐
       │  rubix.analytics.report     │  ──►  starter-export (html/csv/json)
       │  (UndoDispatcher-wrapped)   │  ──►  BlobStore::put
       └─────────────┬───────────────┘
                     │ ChangeDraft(Op::Create)
       ┌─────────────▼───────────────┐
       │  starter-changelog          │
       └─────────────────────────────┘
```

## Verbs

### `rubix.analytics.query`

Read-only. Looks up a named template embedded via `include_dir!`
under [`rubix-tools/src/analytics/templates/`](../../../crates/rubix-tools/src/analytics/templates/),
binds caller-supplied params through ClickHouse's native
`{name:Type}` parameter syntax (no string interpolation), and
returns rows as a JSON array. The catalogue is closed at compile
time, so unknown names surface `rubix.analytics.query.unknown_template`
instead of an opaque ClickHouse error.

Bundled templates:

| Template                    | What it returns                                              |
|-----------------------------|--------------------------------------------------------------|
| `disk_history_weekly`       | per-day avg + peak `percent_used` over the last 7 days       |
| `alert_count_weekly`        | per-day alert counts grouped by severity                     |
| `flow_run_summary_weekly`   | per-flow success / failure / latency summary for 7 days      |
| `user_activity_weekly`      | per-user verb-dispatch counts over 7 days                    |
| `clickhouse_writes_weekly`  | per-table row + byte write totals over 7 days                |
| `undo_count_weekly`         | per-actor undo dispatches over 7 days                        |

### `rubix.analytics.report`

Mints one blob per successful call. The `format` field selects the
exporter (`html` / `csv` / `json`); `pdf` is refused at run time
with `rubix.analytics.report.format_unsupported` because server-side
PDF rendering is deferred to the frontend export path. The returned
locator is namespaced `reports/<template>/<ulid>.<ext>` so the same
`BlobStore` can host multiple report families without collision.

The verb implements `ReversibleTool`: `change_for` emits an
`Op::Create` draft carrying the minted locator under
`kind = "rubix.analytics.report.blob"`. The matching
[`AnalyticsReportReversible`](../../../crates/rubix-tools/src/analytics/report.rs)
inverse deletes the blob through `BlobStore::delete`. `apply_forward`
refuses — re-running the report would mint a fresh blob under a new
locator, which would orphan the changelog snapshot.

## Scheduling

Weekly reports run through the durable scheduler — see
[`design/flows/`](../flows/README.md) for the
`FlowAsService` + `starter_scheduled_flows` contract. The
[`com.rubix.weekly-report`](../../../crates/rubix-flows/flows/weekly-report.yaml)
flow YAML declares `trigger: schedule` + `cron_expr: "0 8 * * 1"`;
the rubix-agent boot path
([`boot::scheduler::spawn`](../../../crates/rubix-agent/src/boot/scheduler.rs))
seeds the schedule on startup and the tick task fires the flow on
Mondays at 08:00 UTC.

## Test coverage

| Layer                                | Test                                                                                                              |
|--------------------------------------|-------------------------------------------------------------------------------------------------------------------|
| `analytics.query` against templates  | [`rubix-tools/tests/analytics_query_test.rs`](../../../crates/rubix-tools/tests/analytics_query_test.rs)          |
| `analytics.report` html / pdf paths  | [`rubix-tools/tests/analytics_report_test.rs`](../../../crates/rubix-tools/tests/analytics_report_test.rs)        |
| End-to-end scheduled fire + undo     | [`rubix-agent/tests/goal_6_weekly_report_test.rs`](../../../crates/rubix-agent/tests/goal_6_weekly_report_test.rs) |

The goal-6 integration test boots testcontainers Postgres
(`starter_scheduled_flows`) + ClickHouse (`system_disk_history`) +
a tempdir-backed `FsBlobStore`, pre-populates seven days of disk
history, advances the `FlowAsService` clock by 7 days to trigger one
fire, asserts the rendered html blob lands with the expected
per-day rows, then calls `rubix.undo.last` and asserts the blob is
deleted.
