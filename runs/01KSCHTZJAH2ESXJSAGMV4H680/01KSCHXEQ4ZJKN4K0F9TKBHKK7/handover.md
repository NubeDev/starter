## Done

- Added `crates/starter-cron/` (Cargo.toml, src/lib.rs barrel, src/error.rs with `CronError::{Parse,Past}`, src/next_fire.rs exposing `pub fn next_fire(now: DateTime<Utc>, expr: &str) -> Result<DateTime<Utc>, CronError>`).
- Wrote `tests/next_fire_test.rs` with 4 cases: weekly Mon 09:00, every-15-minutes, business-hours MON-FRI 09:00, and malformed-expression Parse error path.
- Registered crate in root `Cargo.toml` workspace members.
- `cargo test -p starter-cron` green (4 tests + 1 doctest).
- Committed as `phase A.1 — starter-cron crate — feat(starter-cron) cron parser + next_fire`.

## Next

- Stage 2 of 15 (next session) — continue Phase A toward `scheduled_flows` PG table + FlowAsService schedule trigger.

## What you need to know

- Stage prompt said "depending on the cron crate from crates.io, time, thiserror" but the required signature uses `chrono::DateTime<Utc>`, and the workspace already exposes `chrono` (no `time` workspace dep). I used `cron = "0.12"` + workspace `chrono` + workspace `thiserror`. If a later stage truly needs the `time` crate, add it then.
- `cron` 0.12 grammar is 6–7 fields (`sec min hour dom mon dow [year]`). The lib.rs doc-comment documents this explicitly so downstream callers don't pass POSIX 5-field strings.
- `next_fire` parses on every call (no caching); good enough for one parse per scheduled row per scheduler tick. Note left in the source if perf ever matters.

## Open questions

- (none)
