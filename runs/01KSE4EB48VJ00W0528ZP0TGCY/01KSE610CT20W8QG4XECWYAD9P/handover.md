## Done

- added `rubix/crates/rubix-agent/src/routes/flow_events.rs` — `GET /api/v1/flows/{flow_id}/events` SSE route subscribing through a shared `FlowSubscriptionRegistry`, projecting `FlowEvent::NodeEmitted` through `starter_flow_spi::event_dto::NodeSlotValue`, 15s keep-alive, CSRF-exempt (mounted alongside `/healthz` outside the auth sandwich)
- added `rubix/crates/rubix-agent/src/boot/flow_runtime.rs` — `FlowSubscriptionRegistry` (per-flow `broadcast::Sender<FlowEvent>` map), `build()` selecting `SqliteNodeStateStore` over `~/.rubix/node_state.db` (with `FLOW_MIGRATION_SOURCE` applied) when `RUBIX_DATABASE_URL` + `[flow_runtime].state_db_path` are set, else `InMemoryNodeStateStore`, plus `bundled_schedule_pairs()` extracted from the scheduler
- refactored `boot/scheduler.rs` to delegate bundled-YAML enumeration to `flow_runtime::bundled_schedule_pairs` so the always-on runtime and the durable scheduler share one definition
- extended `AgentConfig` with `[flow_runtime] state_db_path` (default `~/.rubix/node_state.db`) and re-exported `FlowRuntime`, `FlowSubscriptionRegistry`, `build_flow_runtime`, `bundled_schedule_pairs`, `BundledSchedule` from `boot/mod.rs`
- wired the runtime + SSE router into `main.rs`; added cargo deps (`starter-store-sqlite` with `flow` feature, sqlx `sqlite`, `tokio-stream`)
- tests green: `cargo test -p rubix-agent --lib` → 28 passed / 1 ignored, including the two new flow_runtime tests + flow_events projection test
- committed `612c374` on `codeless/rubix-flow-live-tick-demo`

## Next

- stage 7 picks up the engine-side run pump that ties each live `RunHandle::events_tx` into `FlowSubscriptionRegistry::sender(flow_id)` so the SSE route emits real slot values (today it heartbeats while no producer is wired)
- iterate every live revision in `FlowRegistry` and construct `FlowRunner` instances per the stage-6 description — punted because `FlowRegistry` is empty at boot today (no call site populates it; the MCP `FlowAsTool` registration in `boot/mcp/register.rs` owns its own registry) and the engine-side `FlowRunner` plumbing requires another wiring pass

## What you need to know

- Stage-6 scope was partially landed: the seams (registry, NodeStateStore, SSE route, refactored scheduler, config knob) are all live and pass tests, but the engine-side "iterate every live revision and start a FlowRunner per schedule trigger" piece is **not** in this commit. The seam is in place: any producer that grabs `FlowRuntime::subscriptions.sender(flow_id)` and forwards `RunHandle::events_tx.subscribe()` into it lights up the SSE wire shape end-to-end without further API changes.
- `boot::scheduler::spawn` still drives the cron-triggered runs (via `FlowAsService::register_schedule` → MCP `FlowAsTool::invoke`); generalising it to *also* feed events into the subscription registry is the natural follow-up.
- `~`-expansion of `state_db_path` is handled inside `flow_runtime::resolve_state_db_path` (against `$HOME`); parents are `mkdir -p`'d on demand.
- `starter-store-sqlite` is now a normal dep (still also in dev-deps with `testing` feature for test code).

## Open questions

- Should the SSE route be auth-gated like the tools router? Today it's exempt for `EventSource` compatibility (no CSRF, no body) but lives outside the `with_principal` sandwich too — the existing extensions-events SSE pattern is the same, so this matches precedent but is worth re-confirming before the frontend live-view stage consumes it.
- `[flow_runtime].state_db_path` defaults to `~/.rubix/node_state.db` even when no DSN is present; the current `build()` only opens SQLite when *both* the DSN and the path are set. Confirm that's the intended gate (the stage description reads "when RUBIX_DATABASE_URL is set otherwise InMemoryNodeStateStore" — current behaviour matches but couples the two knobs).
