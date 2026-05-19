## Done

- Filled the Decisions section of `.codeless/jobs/starter-flow-scaffold/SCOPE.md` with four scaffold-relevant decisions (D-S1.1 default-features, D-S1.2 per-kind feature table, D-S1.3 baseline location/policy, D-S1.4 starter-ext-flow out-of-scope) plus a recorded D1 deferral. Each decision has an explicit "Revisit when" trigger.
- Committed as `stage 1: lock scaffold decisions` on `codeless/starter-flow-scaffold`.

## Next

- REVIEW gate per WORKFLOW.md — sign-off on the four locked decisions before stage 3 lands the `starter-flow-spi` crate.
- Stage 3 implements `starter-flow-spi` in full (Phase 1 contracts) and creates `DOCS/flow/scope/starter-flow-spi-deps.baseline.txt` from `cargo tree -p starter-flow-spi --edges normal`.
- Stages 4 / 5 / 6 then land empty crate skeletons for `starter-flow`, `starter-flow-nodes` (using the D-S1.2 feature table verbatim), and `starter-flow-surfaces`.
- Stage 7 wires the workspace and runs the dep-tree gates.

## What you need to know

- Stage 1 is prose-only by design (per WORKFLOW.md "Sequencing" and the "What done looks like" row for stage 1). No code touched, no Cargo files edited, no baseline file created yet — that lands in stage 3 when the crate exists.
- The D-S1.2 feature table is the authoritative mapping for stage 5; copy the feature names and kind ids verbatim. Triggers use kebab `trigger-<flavour>` because Cargo feature names cannot contain `.`, while the kind id keeps the dotted `starter.flow.trigger.<flavour>` form per R10.
- D1 (`adk-rust` vs lifted Codeless Runner) is recorded as deferred. The `ai-agent` cargo feature in `starter-flow-nodes` (stage 5) is the switch that will gate the eventual choice; do not import `adk-rust` anywhere in this scaffold.
- The source-of-truth SCOPE (`DOCS/flow/scope/SCOPE.md`) was not modified; per the local SCOPE's header, that doc wins on conflict and these scaffold-job-specific decisions belong in the per-job brief, not in the flow SCOPE itself.

## Open questions

- (none) — D1 is recorded as deferred, not open at the scaffold layer.
