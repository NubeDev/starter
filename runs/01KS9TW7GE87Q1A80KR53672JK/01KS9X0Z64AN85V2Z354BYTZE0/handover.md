## Done

- verified Stage 1 skeleton against Layer-1 invariants (R1 dep direction, R2 single transport, R4/R5 trust boundary, wire-formats)
- ran `cargo build` across all 7 rubix Rust crates — green
- confirmed rubix crates are members of the starter root Cargo.toml (no parallel workspace; pnpm workspace scoped to rubix/ only per Q2)
- exercised mani.yaml lint task with a synthetic 402-line file under rubix/contracts/spi/src/ — it exited rc=1 with the offender named, then removed
- confirmed pnpm workspace linkage: extension-ui-sdk/node_modules/@rubix/ui-core symlinks to ../../../ui-core and node require.resolve succeeds

## Next

- PASS: proceed to Stage 3 (docs / next slice per SCOPE.md)

## What you need to know

- Sentinel: PASS: Stage-1 skeleton holds R1 dep direction (rubix-spi → starter-spi only; all other rubix crates → rubix-spi only), R2 (no transport added in Phase 0), R4/R5 (SPI exposes only reserved module slots, no extra deps), and wire-formats remain untouched; the mani lint task mechanically enforces the R1 400-line budget (verified by injecting a 402-line file).
- No commit made — review stage produced no diffs (the synthetic violator was created, asserted, removed).
- The rubix Cargo.toml does not exist at rubix/ root; rubix crates are listed as members of the parent starter Cargo.toml — this is the intended R0 shape.
- pnpm-workspace.yaml at rubix/ scopes 6 TS packages; cross-package workspace:* deps resolve.

## Open questions

- (none)
