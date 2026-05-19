## Done

- Implemented stage 7: `RunState` (state.rs) and `FlowRunner` + `SkillSelector`/`SkillSelection` + `RunStore`/`InMemoryRunStore` + `RunSpec`/`RunHandle`/`FlowRunnerConfig` (run.rs).
- SkillSelector hook invoked exactly once per `FlowRunner::start`; selection pinned on `RunState` as `Arc<SkillSelection>`.
- Coordinator task emits `RunStarted`, kicks the stage-4 propagator, seeds writes through the single `GraphStore::write_slot` chokepoint, detects propagator terminal events, and emits `RunCompleted` on quiescence with a terminal-slot output map.
- 4 new tests (success / cancel / cycle-exhausted / skill-once) + RunCancel parity tests; full crate test suite 24/24 green; clippy `-D warnings` green.
- Committed as `60b2f33` on `codeless/starter-flow-engine`.

## Next

- Stage 8 per the Phase 2 plan (a fresh session picks it up).

## What you need to know

- `SkillSelection` is a placeholder pending `starter-skills`; carries a free-form `label` only. Drop and re-export the canonical type from `starter_flow_spi::skill` when that crate lands.
- Run completion is detected by a quiescence window (default 100ms with no events) rather than a terminal-node sentinel — keeps Phase 2 simple; revisit if Phase 3 surfaces want stricter semantics.
- `RunStore::record` takes `Arc<RwLock<RunState>>` so the coordinator can keep mutating the same record (no separate save call in Phase 2).
- `RunHandle.initial_rx` is the only no-miss-`RunStarted` receiver path; additional consumers use `events_tx.subscribe()`.

## Open questions

- (none)
