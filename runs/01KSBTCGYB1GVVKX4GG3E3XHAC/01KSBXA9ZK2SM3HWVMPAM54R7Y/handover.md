## Done

- reviewed stage-1 diff (commit 542b918) against stage-2 criteria
- verified grep cleanliness, FlowAsTool-only registration, mcp_tools=6 log, lint clean, serde_yaml usage, LoadError context

## Next

- (none — gate emits FAIL; next session must address the two deviations before re-gating)

## What you need to know

- `docs/design/flows/README.md` lines 99 and 102 use the word "stub" — stage-2 criteria forbid 'stub' wording (alongside 'PR 3', 'placeholder', 'unblocked'). Rewrite the "Block-A scope note" section in present tense or drop it
- `crates/rubix-flows/src/load.rs` is 384 lines vs the ≤150 line target. The bulk is doc-comments + tests; trimming module-doc, collapsing `parse_yaml`/`convert`/`load_all`/`into_arcs`/`walk` paragraphs, and moving the test EXPECTED_FLOW_IDS check into a smaller assertion shape would land it well under the cap
- Layer-1 invariants (R1/R2/R4/R5, wire formats) hold — these are spec-compliance deviations, not invariant breaks

## Open questions

- (none)

FAIL: docs/design/flows/README.md still uses forbidden "stub" wording (lines 99, 102) and crates/rubix-flows/src/load.rs is 384 lines vs the ≤150 cap — both are explicit stage-2 confirmation criteria.
