## Done

- reviewed stage-1 diff (d23ef3a) against Layer-1 invariants R1/R2/R4/R5, wire-formats, R-ins-1, R-ins-9
- confirmed starter-spi gained insights types with no reverse deps; starter-ext-insights-iot depends on starter-spi only (D1 honored)
- confirmed rule.rust and verdict.join are plain NodeBehavior impls — no parallel orchestrator
- confirmed IoT pack uses register_rules / register_quality_flags free functions per contributes.tools pattern

## Next

- PASS: Layer-1 invariants (R1 dep arrows, R2 single transport, R4/R5 trust boundary, wire formats untouched, R-ins-9 no parallel orchestrator, R-ins-1 register() seam) all hold for stage 1.

## What you need to know

- Sentinel below; runtime gates on it
- Forward-stage gap (not blocking now): SCOPE places node-kind id constants + cargo features in starter-flow-nodes with bodies in starter-insights; stage 1 colocated both in starter-insights. Worth wiring the feature-flag stubs in starter-flow-nodes in a later ramp step
- No commit made — this is a review-only stage

## Open questions

- (none)
