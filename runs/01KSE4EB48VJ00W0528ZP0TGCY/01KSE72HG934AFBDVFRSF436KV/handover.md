## Done

- Reviewed Phase C diff (commits d10ef9a, 612c374, bc5993b — 884 +/64 lines across rubix-agent boot/flow_runtime.rs, routes/flow_events.rs, scheduler refactor, flow_live_tick_test.rs)
- Confirmed Layer-1 invariants (R1 dep direction, R2 single SSE transport, R4 NodeSlotValue wire-format untouched, R5 state seam, trust boundary matches extensions-events precedent)
- Confirmed SCOPE OQ-5: subscription registry is per-flow tokio::broadcast::Sender<FlowEvent> keyed by flow_id

## Next

- Phase D (bundled com.rubix.tick-counter YAML) — but the engine-side run pump + always-on FlowRunner mounter punted in Stage 6 must be landed first, otherwise the bundled flow will deploy but never tick in real boot
- Resolve the auth-gating question on the SSE route flagged in Stage 6 OQ before frontend live-view consumes it

## What you need to know

- PASS sentinel covers Layer-1 invariants only — the operator manual flow described in the Stage 8 brief (curl deploy → SSE → climbing sqlite count) does NOT yet succeed against a real make start; flow_live_tick_test fakes the pump by direct Counter::invoke + manual broadcast send
- Three Phase C commits land: C.1 verification-only (no commit, deploy already publishes via NOTIFY → FlowRegistry::register); C.2 seams (registry, SSE route, NodeStateStore wiring, scheduler refactor) — 28 lib tests pass; C.3 integration test — 1 ignored test passes under docker
- `[flow_runtime].state_db_path` is only honoured when RUBIX_DATABASE_URL is also set; the two knobs are coupled (matches stage spec wording but worth noting)

## Open questions

- (none — gate decision recorded below)

PASS: Phase C upholds R1 (upstream-only deps), R2 (reuses axum SSE), R4 (NodeSlotValue DTO unchanged), R5 (state behind NodeStateStore trait), and the trust boundary matches the existing extensions-events SSE precedent; SCOPE OQ-5 resolved as per-flow broadcast keyed by flow_id.
