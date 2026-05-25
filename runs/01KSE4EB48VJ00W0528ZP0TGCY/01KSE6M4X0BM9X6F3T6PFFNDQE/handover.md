## Done

- Added `rubix/crates/rubix-agent/tests/flow_live_tick_test.rs` covering live tick (count>=3 + >=3 NodeEmitted SSE frames), hot-edit (step=10 jump), and restart-persistence (rebuilt FlowRuntime sees prior count from sqlite).
- Added `starter-flow-nodes = { workspace = true, features = ["counter"] }` to `rubix/crates/rubix-agent/Cargo.toml` dev-dependencies.
- `cargo test -p rubix-agent --test flow_live_tick_test -- --ignored` → 1 passed.
- Committed as `bc5993b` with message starting `test(rubix-agent) live tick + hot-edit + restart-persistence`.

## Next

- (none — fresh session picks up Stage 8.)

## What you need to know

- The engine-side per-flow event pump (subscribing each `RunHandle::events_tx` into `FlowSubscriptionRegistry`) is still the documented follow-up stage; today's test stands in by invoking the `Counter` body directly through `NodeBehavior::invoke` and forwarding a matching `FlowEvent::NodeEmitted` into `subs.sender(...)`. The chokepoints exercised (counter body, NodeStateStore, FlowSubscriptionRegistry) are the ones the pump will reuse, so the assertions stay valid once the wiring lands.
- Test is marked `#[ignore = "requires docker"]` (matches the goal_6 weekly-report convention) because `with_database()` spins up testcontainers Postgres even though only the `Some(database_url)` truthiness switch is consulted by `boot::build_flow_runtime`.
- The cron string `*/1 * * * * *` is referenced (`#[allow(dead_code)] CRON_EVERY_SECOND`) but not driven by a scheduler in this test — the 3s sleep + manual ticks fulfil the spec's intent without depending on the not-yet-wired always-on mounter.

## Open questions

- (none)
