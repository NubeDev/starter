## Done

- Added `ProcessStats` (pid, started_at, uptime, rss_bytes, cpu_pct, restarts) and the `ProcessFlavour` discriminator to `starter-ext-spi` in new `src/process.rs`, exported from lib.rs.
- Added a shared live-process cell to `starter-ext-supervisor` (new `src/proc_stats.rs`: `LiveProcess`, `ProcessCell`, `/proc` parsers). The cell is populated next to the existing `EventKind::Spawned` push and cleared on every cycle exit in `run()`.
- Added `SupervisorHandle::pid() -> Option<u32>` and `process_stats() -> Option<ProcessStats>`, both gated on `LifecycleState::Running`.
- RSS/CPU sampled on the existing health tick (`sample_process`) via `/proc/<pid>/stat` + `/statm` on Linux; non-Linux returns `None` (no new timer).
- Added `GET /extensions/{id}/process` in `starter-ext-server` (new `src/process.rs`), wired into router + doc table; returns `ProcessStats` or `404 {code: "ext.process.not_running"}` for builtin/wasm/stopped/unknown.
- Tests: pid set-on-spawn/clear-on-exit integration test in `tests/hello_process.rs`; `/proc` parse + CPU-delta unit tests in `proc_stats.rs`; spi round-trip tests.
- Committed as `63a4b4b`.

## Next

- Stage 3 (metrics): sampled per-extension counters/gauges + `GET /extensions/{id}/metrics`, also piggybacking on the health loop.

## What you need to know

- Workspace clippy under `-D warnings` fails ONLY in pre-existing, untouched `starter-ext-insights-energy` / `starter-ext-insights-finance` crates (`if_same_then_else`, `neg_cmp_op_on_partial_ord`) — verified pre-existing via stash, unrelated to this stage. My three crates are clean under `-D warnings`; fmt --check clean; all tests pass.
- CPU% sampling assumes USER_HZ=100 and PAGE_SIZE=4096 (documented constants; no libc dep per R2). cpu_pct is `None` until the second sample.
- `pid()`/`process_stats()` deliberately return `None` unless state == Running, so a stale pid is never reported during teardown.

## Open questions

- (none)
