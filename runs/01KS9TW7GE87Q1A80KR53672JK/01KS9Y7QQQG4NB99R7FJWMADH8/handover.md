## Done

- Reviewed all 8 design docs in rubix/docs/design/; line counts 223–376, all under the R1 400-line cap
- Verified each doc opens with a "Source: rubix/SCOPE.md ..." citation block naming the specific rules/sections it expands
- Cross-checked OVERVIEW.md's Rust dep arrow against ../Cargo.toml: the nine Phase-0 rubix crates wired there (spi, graph, engine, kinds-registry, data-postgres, data-clickhouse, apps/agent, agent-sdk, agent-client-rs) sit on the arrow correctly; rubix-spi is the only path alias re-exported via [workspace.dependencies], matching R5
- Cross-checked OVERVIEW.md's TS dep arrow against pnpm-workspace.yaml: six packages (agent-client-ts, ui-core, ui-kit, extension-ui-sdk, studio, desktop) match
- Confirmed AUTH.md and MIGRATIONS.md are the two thickest non-VERSIONING docs and each carries an explicit "Phase 1 entry expectation" section — the gate the source SCOPE names

## Next

- (none) — stage 5 of 5 is review-only; no code or commits to land

## What you need to know

- PASS sentinel emitted below. Layer-1 invariants (R1 file size, R2 single-API/no-back-channels in docs only, R4/R5 dependency direction, wire-formats untouched) all hold against the prior-stage diff
- OVERVIEW.md's repo map lists forward-looking domain-* and transport-* crates that are NOT yet in Cargo.toml; this is intentional (the doc says "others land just-in-time before the phase that needs them") and is not drift — only crates already present in Cargo.toml appear in the dep-arrow section
- No file edits made this stage; nothing to commit

## Open questions

- (none)
