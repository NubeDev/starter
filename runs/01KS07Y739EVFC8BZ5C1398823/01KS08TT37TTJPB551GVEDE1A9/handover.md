## Done

- Added `crates/starter-flow/tests/smoke_one_write_chokepoint.rs` — three writers (REST-style tokio task, CLI-style sync call, propagator tick via fed-input transform `EchoBehavior`) all hit `GraphStore::write_slot` for the same `(NodeId, slot)`; a custom `tracing-subscriber` `Layer` counts `write_slot` spans filtered by `node_id`+`slot_name` and asserts exactly 3; a `store.subscribe()` receiver asserts exactly 3 `SlotChanged` envelopes with the three distinct values.
- Added `crates/starter-flow/tests/smoke_engine_is_reader_of_policies.rs` — registers `FailSafeOutput` (declared `fail-safe(Int(0))`) and `HoldLastOutput` (declared safe-state = held value at stop); pre-stop `store.subscribe()` confirms one engine-driven `SlotChanged` per writable carrying the declared safe value; plus a `walk_rs_files` grep-assert over `crates/starter-flow/src/` rejecting `^\s*"<policy>"` for `safe_state`, `session_policy`, `on_failure`, `cost_cap`, `trigger`, `auth`, `timeout` (doc-comment lines skipped).
- Added `tracing-subscriber` to `starter-flow` dev-dependencies only.
- `cargo test -p starter-flow --tests` → 27 passed; `cargo clippy -p starter-flow --tests --all-features -- -D warnings` clean; `cargo fmt -p starter-flow -- --check` clean.
- Committed as `33e564e` on branch `codeless/starter-flow-engine-finish`.

## Next

- Stage 6+ should add the separate R3 grep-contract test at `crates/starter-flow/tests/r3_no_policy_match_arms.rs` (locked in stage 1 decisions; deliberately not landed here per the SCOPE-smoke vs R3-contract split called out in the stage prompt).

## What you need to know

- The SCOPE-smoke grep test in `smoke_engine_is_reader_of_policies.rs` and the future R3 contract test cover overlapping ground but live in separate files for blast-radius granularity, as called out in this stage's prompt and stage-1 decisions.
- The `WriteSlotCounter` Layer extracts `node_id` / `slot_name` via both `record_str` and `record_debug` (Display values pass through `record_debug` with quoted Debug formatting, which is stripped). Counter is scoped to spans whose name == `"write_slot"`.
- `HoldLastOutput::safe_state` is sync per the `WritableOutput` trait; the smoke uses `try_lock` with a `Null` fallback — never contended because the test does not hold the lock concurrently with the engine's stop walk.
- Integration tests rely on `tracing_subscriber::registry().with(layer).try_init()` so re-runs in the same process do not panic.

## Open questions

- (none)
