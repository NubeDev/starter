## Done

- Implemented stage 6: engine state machine in `crates/starter-flow/src/engine.rs` per R12. `EngineState` enum + `can_transition_to` matrix, `Engine` struct (GraphStore, registries, watch::Sender state, propagator JoinHandle, writable-output list), `start/stop/pause/resume` with info-span + info-log per transition, `WritableOutput` trait + safe-state walk on `stop`, SIGTERM documented as bin-level concern.
- 5 new unit tests cover the exhaustive transition matrix, full happy-path + illegal-call typed errors, Starting→Stopped fast-path, the safe-state walk via a recording fake writable, and the state watch surface. `cargo test -p starter-flow` = 20/20 green; `cargo clippy -p starter-flow --all-targets -- -D warnings` clean.
- Committed as `f04cd14`.

## Next

- Stage 7 picks up next per the job plan (per-flow Cancel + FlowEvent stream + FlowRunner wiring leading into the two built-in node kinds).

## What you need to know

- `Engine::stop` joins the propagator via `handle.abort(); handle.await` (best-effort) — when stage 7 wires `FlowRunner`, decide whether to flip a per-run Cancel before abort.
- `WritableOutput` default impl uses `WriteSlotOpts::live()` so safe-state writes DO publish `SlotChanged` per the R2 explicit callout — keep that flag visible if a future override re-routes.
- `Engine::set_propagator` is the hook for tracking the latest propagator handle (replaces any previous one); per-run handle tracking is deferred to the run/runner stage.

## Open questions

- (none)
