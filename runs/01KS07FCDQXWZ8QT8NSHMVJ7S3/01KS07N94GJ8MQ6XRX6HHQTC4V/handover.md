## Done

- Reviewed stage 1 diff (commit 984a262): 56-line addition to DOCS/user/scope/SCOPE.md under "Phase 0 wire boundary (locked)" recording closed-enum membership for Quantity, Unit, Theme, DateFormat, TimeFormat, WeekStart, NumberFormat, UnitSystem with per-rule revisit triggers.
- Verified Layer-1 invariants R1/R2/R4/R5 hold and no wire-format code was emitted.
- PASS: closed-enum membership decisions are documented for all 8 enums with canonical-SI mapping consistent with R1, Money correctly excluded from Unit, Ratio deferral explicitly tied to a future Phase 1 resolver rule, and no Layer-1 invariant was disturbed.

## Next

- Stage 3 picks up the next WORK step (first code landing in starter-spi: the preferences DTOs and/or units module per the locked enum boundary).

## What you need to know

- This was a REVIEW gate, not a code stage. Sentinel below is the runtime's hook.
- Stage 1 was documentation-only; nothing to clippy/test.
- The locked enum names in SCOPE.md use Rust-style CamelCase variants (e.g. `Kpa`, `MPerS`, `KmPerH`, `IsoYMD`, `DmySlash`, `H24`). Stage 3 code should bind to those exact spellings or revise SCOPE.md before deviating.
- `Ratio` is intentionally absent from v1 Quantity; the `{Ratio, Percent}` slot-pair rejection lives in Phase 1 resolver, not in starter-spi.

## Open questions

- (none)

PASS: stage 1 locks the eight closed enums with canonical SI matching R1, Money excluded, Ratio deferral documented, and no Layer-1 invariant disturbed.
