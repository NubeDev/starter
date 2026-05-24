# SCHEDULING — durable cron triggers for bundled flows

Cron-triggered flows in rubix run through `FlowAsService`, the
service-surface companion to `FlowAsTool`. A schedule is one row
in the `scheduled_flows` Postgres table; a single tick loop in
each rubix-agent process claims due rows under
`SELECT FOR UPDATE SKIP LOCKED` and dispatches them through the
same `FlowRunner` that handles explicit invocations. Cron parsing
lives in [`starter-cron`](../../../../crates/starter-cron/) and
accepts 5-, 6-, and 7-field expressions.

> **Cites:** SCOPE [Phase 4](../../SCOPE.md), GAPS row 16
> (addressed), [docs/design/flows/](../flows/README.md), and the
> upstream ledger entries under
> [docs/design/starter-changes/](../starter-changes/README.md).

## Why durable, not in-process

The first cut of weekly-report stored its schedule in a `tokio`
interval inside `rubix-agent`. Three things broke that:

1. **Process restart loses the next-fire time.** A weekly report
   that misses its window because someone redeployed on Monday
   morning is the worst kind of silent miss.
2. **Multi-instance.** Two rubix-agent replicas behind the same PG
   would both fire the same schedule. There was no shared lease.
3. **The Phase 4 audit story.** Every fire must land a `changelog`
   row with the schedule id; an in-process timer has nothing to
   key off.

`scheduled_flows` solves all three. The row *is* the schedule.
`next_run_at` is the only authority on when. `SELECT FOR UPDATE
SKIP LOCKED` is the lease.

## Components

```
┌─────────────────────────────────────────────────────────────────┐
│  rubix-agent::main (boot)                                       │
│                                                                 │
│   FlowRegistry  ◄──── rubix_flows::load_all()                   │
│        │                                                        │
│        │ for every YAML with trigger: schedule                  │
│        ▼                                                        │
│   FlowAsService::register_schedule(tenant_id, flow_id, cron)    │
│        │                                                        │
│        ▼                                                        │
│   FlowAsService::start()  ───►  spawn tick task (tokio)         │
└─────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼   every tick_interval_seconds
                       ┌─────────────────────────────────────────┐
                       │  tick()                                 │
                       │                                         │
                       │  BEGIN                                  │
                       │    SELECT * FROM scheduled_flows        │
                       │      WHERE next_run_at <= now()         │
                       │        AND enabled = true               │
                       │      FOR UPDATE SKIP LOCKED LIMIT 32    │
                       │                                         │
                       │    for row in claimed:                  │
                       │      FlowRunner::run(flow_id, ctx)      │
                       │      UPDATE scheduled_flows SET         │
                       │        last_run_at = clock.now(),       │
                       │        last_run_status = …,             │
                       │        last_run_message = …,            │
                       │        next_run_at =                    │
                       │          starter_cron::next_fire(       │
                       │            cron_expr, clock.now())      │
                       │      WHERE id = $id                     │
                       │  COMMIT                                 │
                       └─────────────────────────────────────────┘
```

## `scheduled_flows` table

Owner: `starter-store-postgres`, migration `scheduled_flows/0001_init.sql`.

| Column | Type | Notes |
|---|---|---|
| `id` | `TEXT PRIMARY KEY` | ULID; stable identity for changelog references |
| `tenant_id` | `TEXT NOT NULL` | tenant scope; tick query filters by current tenant set |
| `flow_id` | `TEXT NOT NULL` | matches `FlowRegistry` entry |
| `cron_expr` | `TEXT NOT NULL` | accepted by `starter_cron::parse` (5/6/7 fields) |
| `next_run_at` | `TIMESTAMPTZ NOT NULL` | the only authority on "due now"; the tick query indexes on this |
| `last_run_at` | `TIMESTAMPTZ NULL` | last fire (success or failure) |
| `last_run_status` | `TEXT NULL` | `success` / `error` / `skipped` |
| `last_run_message` | `TEXT NULL` | error message or empty |
| `enabled` | `BOOLEAN NOT NULL DEFAULT true` | toggle without dropping the row |
| `created_at` | `TIMESTAMPTZ NOT NULL DEFAULT now()` | |
| `created_by` | `TEXT NULL` | actor that registered the schedule |

`UNIQUE (tenant_id, flow_id)` enforces "one schedule per flow per
tenant". A `pg_notify('starter_scheduled_flows', …)` trigger fires
on insert and on update of `next_run_at` or `enabled`, so a future
sidecar (or a different rubix process) can react without polling.

## Cron grammar — `starter-cron`

The previous in-tree parser accepted only 5-field expressions and
rejected `weekly-report.yaml`'s `0 8 * * 1`-shaped variants when
they were normalised to a 6-field form by upstream tooling. The
new [`starter-cron`](../../../../crates/starter-cron/) crate
accepts:

- **5 fields** — `m h dom mon dow` (POSIX)
- **6 fields** — `s m h dom mon dow` (Quartz-lite, second-precision)
- **7 fields** — `s m h dom mon dow year` (Quartz)

Field overflow (e.g. `dow=7`) is normalised, not rejected. The
public surface is:

```rust
pub fn parse(expr: &str) -> Result<Schedule, CronError>;
impl Schedule {
    pub fn next_fire(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>>;
}
```

`FlowAsService::tick()` calls `next_fire(after = clock.now())`
after every successful or failed dispatch so a missed window
recovers on the next tick rather than back-firing.

## Clock injection

`FlowAsService` holds an `Arc<dyn Clock>`. Production wires
`SystemClock`; integration tests wire `TestClock`, advancing it
deterministically. The Goal 6 test
([rubix-agent/tests/goal_6_weekly_report_test.rs](../../../crates/rubix-agent/tests/goal_6_weekly_report_test.rs))
advances the clock by 7 days and asserts exactly one fire — that
is the only reason this code is testable without time.

## Boot-time seeding

`rubix-agent` walks every bundled flow YAML and, for each carrying
`trigger: schedule`, calls
`FlowAsService::register_schedule(tenant_id, flow_id, cron_expr)`.
The register is idempotent (`ON CONFLICT (tenant_id, flow_id) DO
UPDATE SET cron_expr = $cron, enabled = true`), so reseeding on
every boot is safe and "edit the YAML, restart the agent" is the
canonical way to change a schedule today. Manual `register_schedule`
calls from operator tooling stay on the same path — there is no
second writer.

## Config

`[scheduler]` section in `AgentConfig`:

```toml
[scheduler]
enabled = true             # default true; false skips the spawn
tick_interval_seconds = 60 # default 60
```

When `enabled = false`, schedules still seed; only the tick task
is skipped. This is what local development and CI integration
tests use when they don't want background fires.

## Failure model

- **Cron parse failure at boot.** The seeder logs an error and
  drops the row from the seed set; the agent boots without that
  schedule. The bundled YAML is the only acceptable fix.
- **`FlowRunner::run` returns `Err`.** The row is updated with
  `last_run_status = "error"` + `last_run_message`; `next_run_at`
  still advances. The next tick re-fires per the cron, not
  immediately. This matches "weekly means weekly" — we don't
  hammer on errors.
- **Process crash mid-tick.** The `FOR UPDATE` lock releases on
  connection drop; another tick (in the same or another process)
  picks the row up. At-least-once semantics are explicit; the
  flow body is expected to be idempotent at the granularity of a
  single fire window.

## What this does not do

- No second-precision scheduling drift control beyond `next_fire`.
- No catch-up: if `next_run_at` is six fires in the past, one fire
  happens and `next_run_at` advances to the next future boundary.
  Operators wanting backfill use a one-shot `trigger: explicit`
  invocation.
- No leader election beyond `SELECT FOR UPDATE SKIP LOCKED`. Two
  replicas race for each row; the loser skips it.

## Pointers

- Code: [`crates/starter-flow-surfaces/src/service.rs`](../../../../crates/starter-flow-surfaces/src/service.rs)
- Migration: [`crates/starter-store-postgres/migrations/scheduled_flows/0001_init.sql`](../../../../crates/starter-store-postgres/migrations/scheduled_flows/0001_init.sql)
- Cron: [`crates/starter-cron/`](../../../../crates/starter-cron/)
- Test fixture: [`crates/starter-flow-surfaces/tests/scheduled_flows_tick_test.rs`](../../../../crates/starter-flow-surfaces/tests/scheduled_flows_tick_test.rs)
- Boot wiring: [`rubix-agent/src/main.rs`](../../../crates/rubix-agent/src/main.rs)
- End-to-end test: [`rubix-agent/tests/goal_6_weekly_report_test.rs`](../../../crates/rubix-agent/tests/goal_6_weekly_report_test.rs)
