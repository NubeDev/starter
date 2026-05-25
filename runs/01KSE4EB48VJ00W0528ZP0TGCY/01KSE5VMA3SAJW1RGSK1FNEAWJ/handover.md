## Done

- Verified Phase A+B Layer-1 invariants against the diff 4ccb639..6d2df96 — R1 dep direction, R2 upstream-first, R4/R5 trust boundary, and the event_dto wire format are all intact
- Confirmed the three A+B commits (291d2fd, 652334e, 6d2df96), the test files (in-mem + sqlite matrices + counter_invoke), the COUNTER_SPEC builtin, and the present-tense rewrites of hot-reload.md + settings.md
- Confirmed SCOPE OQ-1 (size caps wired as KeyTooLarge/ValueTooLarge), OQ-2 (counter uses put with CAS documented as upgrade), OQ-3 (NodeBehavior::on_redeploy default no-op + counter override)

## Next

- PASS: Layer-1 invariants (R1 dep direction, R2 upstream-first, R4/R5 trust boundary, wire-formats untouched) hold across Phase A+B and SCOPE OQ-1/2/3 are reconciled in the landed code; the next session begins Phase C (rubix-side SSE route + always-on mounter)

## What you need to know

- This was a review-only gate; no code or commits were produced this stage
- starter-flow-nodes' counter feature is gated behind `--features counter`; `all-kinds` includes it
- Engine wires NoopNodeStateStore by default in FlowRunner; the real Sqlite impl gets swapped in by rubix-agent in Phase C

## Open questions

- (none)
