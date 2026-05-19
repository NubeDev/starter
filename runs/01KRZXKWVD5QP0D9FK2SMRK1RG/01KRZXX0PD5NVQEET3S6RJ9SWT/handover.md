## Done

- reviewed stage 0 + stage 1 diffs (.codeless/jobs/starter-flow-scaffold/SCOPE.md Decisions block; no code yet)
- checked R1/R2/R4/R5 + wire-format invariants against the four locked decisions (D-S1.1 default-features, D-S1.2 per-kind features, D-S1.3 baseline path, D-S1.4 starter-ext-flow OUT) and the explicit D1 deferral
- emitted stage 2 gate sentinel

## Next

- stage 3: land the starter-flow-spi crate (Phase 1 contracts — traits + types + re-exports, zero runtime logic, depends only on starter-spi) and the baseline file at DOCS/flow/scope/starter-flow-spi-deps.baseline.txt per D-S1.3
- stages 4–6: empty crate skeletons for starter-flow, starter-flow-nodes (with the D-S1.2 feature table), starter-flow-surfaces, each with default-features=[]
- stage 7: workspace build + cargo-tree diff against the stage-3 baseline

## What you need to know

- PASS: doc-only diff, decisions defend R1/R2/R4/R5 and keep starter-ext-flow out per extensions R13; no code yet so no invariant can have regressed
- this is a REVIEW gate — no commit needed (no file changes); the sentinel above is the deliverable the runtime parses
- the trigger-<flavour> Cargo-feature naming in D-S1.2 is intentional (Cargo features cannot contain `.`); kind ids keep the dotted starter.flow.trigger.<flavour> form per R10

## Open questions

- (none — D1 stays deferred to Phase 4 per SCOPE Open-questions, not a stage-2 concern)
