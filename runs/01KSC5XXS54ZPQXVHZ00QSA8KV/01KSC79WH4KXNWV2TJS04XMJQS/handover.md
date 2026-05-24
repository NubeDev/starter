## Done

- Verified Phase A commits b1ffe51 (starter-undo dispatch helper), f56a712 (rubix-tools UndoDispatcher + rubix.undo.last), 3d59cb4 (rubix-agent undo dispatch integration test), 691adab (rubix-store-postgres undo_snapshots migration), a692aa6 (rubix-agent sweep + [undo] config), 065e36a (rubix-flows allowed_tools[] full-list).
- Re-ran: starter-undo lib (2 passed), rubix-agent undo_dispatch_test (1 passed), rubix-flows (3 passed incl. allowed_tools_multi_entry). undo_sweep_test is #[ignore] (testcontainers) — not run here, matches existing PG-test convention.
- Confirmed Open Question 1: starter-undo shipped only the Reversible trait, so the upstream-first change WAS required; record_if_reversible + ChangeDraft + undo_last landed in crates/starter-undo first, then consumed downstream in dependency order.
- Layer-1 invariants hold: R1 dep direction starter→rubix only; R2 upstream-first honoured for starter-undo; R4 Diagnostic/typed outputs preserved; wire formats untouched (allowed_tools reader extends parse only, AiAgentNode already consumed the slot).

## Next

- Phase B.1 (Goal 2 user-admin: first verb pair user/create + user/disable with Reversible + MessageKeys + test) per WORKFLOW stage 5.

## What you need to know

- PASS: Phase A infrastructure landed cleanly across 6 commits; undo dispatch loop, snapshots migration + retention sweep, and bundled-flow allowed_tools[] all green; upstream-first for starter-undo confirmed necessary and applied before consumers.
- undo_sweep_test gated by Docker (#[ignore]); CI will need a docker-test job to exercise it.
- UndoDispatcher opens its own ChangeRecorder::transaction per call — multi-row tool effects sharing a GroupId need a different entry point later (flagged in A.1 handover, not blocking).

## Open questions

- (none — Open Question 1 from SCOPE is resolved: upstream change was needed and is landed)
