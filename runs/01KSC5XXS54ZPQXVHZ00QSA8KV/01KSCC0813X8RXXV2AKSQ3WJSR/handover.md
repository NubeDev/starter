## Done

- Reviewed Phase D commits (stages 11–14) for Layer-1 invariants
- Confirmed goal_3_flow_programmer_test passes
- Verified rubix-tools dep graph stays downward (R1), Tool-trait dispatch via UndoDispatcher (R2), DTOs in rubix-spi / dispatch in rubix-tools (R4/R5), YAML stored verbatim and re-read on NOTIFY (wire formats)

## Next

- Phase E (whatever the ramp's closing stage prescribes — smoke-test session note + handover roll-up)
- Follow-up: swap InMemoryFlowDefStore for the PG-backed impl in boot::mcp::register so goal_3 integration runs through PG end-to-end

## What you need to know

- PASS: Layer-1 invariants (R1/R2/R4/R5 + wire formats) hold across Phase D commits; goal_3 integration test green
- Operator manual flow per the gate brief: curl tools/call rubix.flow_ops.duplicate → SELECT from flows_definitions WHERE flow_id = target AND superseded_at IS NULL (1 row) → curl tools/list (both source + duplicate surface) → curl rubix.undo.last → SELECT (row now has superseded_at set) → curl tools/list (only source surfaces). Today this round-trips against InMemoryFlowDefStore via the Tool seam; against PG once the boot wiring follow-up lands.
- Phase D.1 migration + NOTIFY trigger + flows_seed + flow_notify listener are in place; only the verb-side FlowDefStore PG impl + agent-boot wiring remain to make the test PG-backed.

## Open questions

- None for the gate. Open implementation gap (PG-backed FlowDefStore wire-up) is tracked in the test header comment and design doc.
