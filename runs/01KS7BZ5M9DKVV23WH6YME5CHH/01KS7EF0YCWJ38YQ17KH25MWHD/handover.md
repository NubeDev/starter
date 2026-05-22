## Done

- Reviewed Phase 2 diff (d23ef3a..HEAD) against SCOPE D5 and R-ins-4
- Read rhai_sandbox.rs, rollups.rs, migration 0002, energy_smoke.rs, and grepped retroactive-flag usage

## Next

- (none) — gate FAILED; remediation belongs in a later ramp step per stage instructions

## What you need to know

- D5 step 1 unimplemented: `retroactive-correction@1` flag is only declared in starter-spi; nothing attaches it to Verdicts produced from windows overlapping a mutated input. No `mutated_at` watermark seam.
- D5 step 2 unimplemented: `rollup_checkpoint.last_at_ms` is a single scalar per (rule, window_class). SCOPE requires the checkpoint to become a per-window watermark for retroactive rules.
- D5 step 2 worse: `drain_invalidations` (rollups.rs:333-352) only DELETEs queue rows; it never re-aggregates the invalidated windows from verdict_log. Stale rollup rows are cleared (`stale_since_ms = NULL`) only as a side-effect when an unrelated new verdict lands in the bucket via `fold_one`/`bump`.
- The `d5_retroactive_invalidation_marks_stale_and_drains` test in energy_smoke.rs asserts only that re-tick runs; it never asserts the rollup row is re-aggregated to the corrected total. Test passes against the broken impl.
- R-ins-4 sandbox profile is locked down (Engine::new + disable eval/import/export + caps), but the CI fixture of known-bad scripts (DoS, escape attempts, package imports) called out at SCOPE lines 429-431 is missing, and the string/array/map/expr-depth caps have no smoke coverage.
- Layer-1 invariants (R1 dep direction, R2 transport, R4/R5 trust boundary, wire formats) look intact; nothing in Phase 2 reversed an arrow or introduced a parallel orchestrator.

## Open questions

- FAIL: D5 retroactive flag is unattached, rollup checkpoint is still a single last_at with a drain that does not re-aggregate invalidated windows, and the R-ins-4 known-bad-script fixture is missing.
