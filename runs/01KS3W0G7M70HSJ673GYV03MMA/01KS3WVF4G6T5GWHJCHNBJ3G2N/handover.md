## Done

- Reviewed stage 0 (handover note) and stage 1 (DOCS/agent/SKILLS.md Decisions section) diffs
- Confirmed no code, no new crate, no wire-format, no transport, no adapter edits landed yet
- Verified S-D1/S-D2/S-D3/S-D5 decisions align with R1/R2/R4/R5 invariants

## Next

- Stage 3 picks up the next prerequisite-decisions / scaffolding step per WORKFLOW.md

## What you need to know

- Stages 0–1 are prose-only; the rulebook invariants are trivially intact because no executable surface changed
- S-D5 explicitly defers any R4 byte-exact rewrite to a separate agent-SCOPE-level edit, so this job stays inside the existing trust boundary

## Open questions

- (none)

PASS: Prose-only stages 0–1 add the locked Decisions section without touching crate deps, transport, wire format, or the R4/R5 trust boundary.
