# Proposal: Rebuild Missing Warehouse Tools & Fix Scheduler Freeze

**Status:** Proposed  
**Date:** 2026-05-27  
**Author:** NubeDev  
**Relates to:**
- [warehouse-engine-swap.md](./warehouse-engine-swap.md) — the parent proposal that removed ClickHouse
- [/.codeless/jobs/warehouse-engine-swap/SCOPE.md](/home/user/code/rust/starter/.codeless/jobs/warehouse-engine-swap/SCOPE.md) — the job scope (stage 3 deleted the old tools)

## Problem

Stage 3 of the warehouse-engine-swap deleted the ClickHouse-backed
`rubix.warehouse.clean_minute` and `rubix.warehouse.rollup_15m` tools, but left
the flow YAMLs that reference them in place:

- `crates/rubix-flows/flows/data-flow-cleaner.yaml` → calls `rubix.warehouse.clean_minute`
- `crates/rubix-flows/flows/data-flow-rollup.yaml` → calls `rubix.warehouse.rollup_15m`

These flows fire every 5 seconds / 5 minutes via the durable scheduler. Each
tick logs a WARN ("tool_id not registered") but otherwise completes quickly.

**The actual freeze** is caused by a compounding issue in the scheduler's tick
loop (`starter-flow-surfaces/src/service.rs:tick()`):

1. The tick loop dispatches all due flows **sequentially** in a `for` loop.
2. The `com.rubix.data-flow.producer` flow calls `rubix.warehouse.ingest`,
   which writes to the TimescaleDB `samples` hypertable via the **same PgPool**
   the scheduler uses for `FOR UPDATE SKIP LOCKED` claims.
3. With cron expressions set to `*/5 * * * * *` (every 5 seconds) and the
   scheduler tick interval at 60 seconds, a single tick claims ~12 due
   invocations of each 5-second flow. That's ~36 sequential tool invocations
   (producer × 12 + cleaner × 12 + tick-counter × 12) dispatched one by one.
4. Each `ingest` call acquires a pool connection. If the pool is small (dev
   default is 5) and the tick's own transaction hasn't released its connection
   yet, the pool exhausts. The HTTP server (including `/health`) shares this
   pool and hangs waiting for a connection — the agent appears frozen.
5. Even when pool exhaustion doesn't trigger, the 36 sequential invocations
   take long enough that the *next* 60-second tick fires while the previous
   one is still running. `MissedTickBehavior::Skip` prevents pile-up of tick
   *tasks*, but the single-threaded dispatch within one tick remains the
   bottleneck.

## Fix (two parts)

### Part A — Remove or stub the orphaned flows

**Option A1 (recommended):** Delete the two YAML files outright. The cleaner
and rollup concepts are unnecessary under TimescaleDB — `time_bucket()` queries
and continuous aggregates replace them. The proposal explicitly states "no rollup
table is needed" (see `rubix-tools/src/warehouse/mod.rs` header comment).

**Option A2:** Replace the `tool_id` in each YAML with a no-op tool
(`starter.flow.noop` or similar) so the flow shapes remain as documentation
but produce no WARN spam.

### Part B — Harden the scheduler against pool starvation

1. **Separate pool for the scheduler.** Give `FlowAsService` its own
   `PgPool` (or at minimum its own sub-pool / semaphore-bounded slice) so
   tick bookkeeping can never compete with tool-invocation writes for
   connections.

2. **Concurrent dispatch with bounded parallelism.** Replace the sequential
   `for` loop with a `futures::stream::iter(...).for_each_concurrent(N, ...)`
   (where N = 4 or configurable). This bounds wall-clock time per tick and
   makes the fast-failing flows (cleaner, rollup) not block behind slow ones
   (producer ingest).

3. **Per-invocation timeout.** Wrap `runner.run(...)` in
   `tokio::time::timeout(Duration::from_secs(30), ...)`. A stuck tool
   invocation should fail the individual flow run, not freeze the entire
   scheduler.

4. **Reduce cron frequency of dev flows.** `*/5 * * * * *` (every 5s) with a
   60s tick means 12 runs pile up per tick. Either:
   - Lower the tick interval to 5s (matches the cron), or
   - Raise the cron to `*/60 * * * * *` so at most 1 invocation per tick, or
   - Cap the `LIMIT 32` in the claim query to a lower per-flow limit.

## Immediate workaround (before the fix lands)

Disable the two broken flows in the database:

```sql
UPDATE starter_scheduled_flows
   SET enabled = FALSE
 WHERE flow_id IN (
   'com.rubix.data-flow.cleaner',
   'com.rubix.data-flow.rollup'
 );
```

Or delete the YAML files and restart the agent (the seeder won't re-insert
rows for flows that no longer exist in the bundle).

## Stages

| Stage | Work | Outcome |
|-------|------|---------|
| 1 | Delete `data-flow-cleaner.yaml` and `data-flow-rollup.yaml` | No more WARN spam, fewer flows per tick |
| 2 | Add per-invocation timeout (30s) in `FlowAsService::tick` | Frozen tools can't freeze the agent |
| 3 | Switch sequential dispatch to bounded-concurrent | Tick wall time drops from O(n) to O(n/4) |
| 4 | (Optional) Separate scheduler pool | Belt-and-braces against pool starvation |

## Out of scope

- Rebuilding `clean_minute` / `rollup_15m` as TimescaleDB continuous
  aggregates. The current architecture uses `time_bucket()` at query time;
  continuous aggregates are a performance optimisation for later.
- Changing the `data-flow-producer` synth data generation (that flow works
  correctly today).
- Touching the MCP tool registry or flow-as-tool wiring beyond adding the
  timeout wrapper.
