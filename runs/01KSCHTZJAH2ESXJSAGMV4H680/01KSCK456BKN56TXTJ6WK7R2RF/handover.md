## Done

- Added `FlowAsService::tick()` in `crates/starter-flow-surfaces/src/service.rs`: claims up to 32 due rows under `SELECT … FOR UPDATE SKIP LOCKED LIMIT 32`, recomputes `next_run_at` via `starter_cron::next_fire` inside the claim tx (peer-safe), dispatches each through the bound `FlowRunner` outside the tx, and writes `last_run_at` / `last_run_status` (`succeeded` | `failed`) / `last_run_message` (truncated to 4 KB on a UTF-8 char boundary).
- Added `FlowAsService::start(self) -> JoinHandle<()>`: consumes self into an `Arc`, spawns a `tokio::time::interval(60s)` loop with `MissedTickBehavior::Skip`, logs claim count / errors but never exits on a tick failure.
- Added integration test `crates/starter-flow-surfaces/tests/scheduled_flows_tick_test.rs` (`#[ignore]`, testcontainers PG): registers an every-minute cron, advances `TestClock` 2 × 60s, asserts two dispatches landed on a counting `FlowRunner`, bookkeeping columns populated, `next_run_at` re-armed, third tick at same clock claims zero.
- `cargo test -p starter-flow-surfaces --no-run` succeeds.
- Committed as `0783882` with message starting `stage 6: phase B.2 — FlowAsService tick`.

## Next

- (none — next session picks up stage 7)

## What you need to know

- `tick()` returns `Result<usize, ServiceError>` (the claim count). The `start()` loop swallows the count and logs.
- `start()` consumes `self`; production callers will need to keep the returned `JoinHandle` to `abort()` on shutdown.
- Bookkeeping update is best-effort (logs warn, does not propagate) — a flaky PG must not abort the whole tick loop mid-batch.
- Test uses 6-field cron (`sec min hour dom mon dow`) per existing `register_test.rs` precedent.

## Open questions

- (none)
