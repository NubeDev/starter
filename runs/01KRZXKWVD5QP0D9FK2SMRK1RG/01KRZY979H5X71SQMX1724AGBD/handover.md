## Done

- crates/starter-flow/Cargo.toml: depends on starter-flow-spi (default-features=[]) and workspace tokio with explicit features=["rt-multi-thread"]; doc comments call out the SCOPE phasing + R2 reasons
- crates/starter-flow/src/lib.rs: declares `graph`, `registry`, `propagator`, `engine`, `run`, `state` modules, each with a one-line SCOPE-section + Phase-N doc comment
- six empty module files (graph.rs / registry.rs / propagator.rs / engine.rs / run.rs / state.rs), each containing only a module-level doc comment pointing at the owning SCOPE section + Phase-N marker
- `cargo check -p starter-flow` green
- committed as 9be1d00 with stage-4 title

## Next

- stage 5 (next session): starter-flow-nodes crate skeleton with per-node-kind cargo features (transform, tool-call, ai-agent, branch, merge, gate, subflow, trigger-explicit, trigger-event, trigger-schedule, trigger-webhook, http-out, log, sleep) per the stage-0 decision

## What you need to know

- workspace tokio already enables rt-multi-thread; the explicit feature in starter-flow/Cargo.toml is kept for grep-visibility (commented as such)
- module skeleton uses flat `x.rs` files (not `x/mod.rs`); the SCOPE allowed either
- registry module is documented as landing in Phase 3 (registry plumbing), not Phase 2 — matches the SCOPE "What lands in starter-flow" block
- run module is documented as landing in Phase 2 (lifecycle + Cancel) AND Phase 7 (checkpoint persistence on Pause/Stopped), matching the three-level-stop phasing
- `#![warn(missing_docs)]` is on in lib.rs; future engine work needs doc comments on all pub items

## Open questions

- (none)
