## Done

- reviewed stage 1 diff (commit 2628f17): DOCS/user/scope/SCOPE.md + TODO.md + capture-baseline.sh only; zero code or Cargo.toml changes
- verified locked decisions are consistent with R1/R2/R4/R5/R6: D-2.2 keeps UnitsCtx::convert at handler layer (not body-rewriting), D-5.1 keeps diagnostics rewriter feature-gated default-off and scoped to declared envelope paths, D-0.1/0.2 keep new starter-spi deps behind default-off features so crate-dep direction stays clean
- emitted PASS sentinel

## Next

- stage 3 picks up the substantive Phase 0 closure work: implement the uom/icu_locale_core feature-gating on starter-spi Cargo.toml and re-capture DOCS/user/scope/starter-spi-deps.baseline.txt against the default build

## What you need to know

- PASS: stage 1 is docs+script only; Layer-1 invariants (R1 dep direction, R2 single transport, R4/R5 trust boundary, wire formats) cannot be violated by these changes and the locked decisions explicitly preserve them
- no working-tree changes in this review stage; nothing to commit

## Open questions

- (none)
