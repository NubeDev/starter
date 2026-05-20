## Done

- Created `DOCS/agent/SKILLS.md` (new file) with a `## Decisions` section recording S-D1 (CLI out of scope, flagged for starter-cli), S-D2 (file:// only in v1, parse-fail on other schemes), S-D3 (best-effort model_hint pass-through with WARN log + fallback metric), S-D5 (keep R4 byte-for-byte normalisation; dropping it is a SCOPE-level edit).
- Each decision includes a "Revisit when" trigger and, where relevant, normative shape (error type for S-D2, log/metric shape for S-D3, transform list for S-D5).
- Added a decision-change protocol section so future stages know a decision moves only via a prose-only stage that names the trigger that fired.
- Committed on branch `codeless/starter-skills` as `ed5680d` with message starting `stage 1: pin S-D1/S-D2/S-D3/S-D5 decisions in SKILLS.md`. Not pushed (no push instruction in this stage's brief and no upstream verification done; WORKFLOW closing-trio mentions a push but the stage prompt did not).

## Next

- Stage 2 (next session): Phase 1 — scaffold the `starter-skills` crate skeleton + `SKILL.md` parser per per-job SCOPE.md §Phasing. Land the dep-tree CI gate with Phase 1.
- Consider whether to push `codeless/starter-skills` to origin before Phase 1 begins (closing-trio in WORKFLOW.md item 3 says push every stage; this session did not push — verify branch tracking and push if appropriate).

## What you need to know

- `DOCS/agent/SKILLS.md` did **not** previously exist in the repo. The per-job SCOPE references it as "source of truth" and references R-skills-1…R-skills-8 etc., but the normative body has not been lifted into SKILLS.md yet. Stage 1's brief only required the Decisions section, so the file was created with: (a) a header pointing to per-job SCOPE for the normative body until a follow-on prose stage lifts it, (b) the Decisions section, (c) the decision-change protocol. A follow-on prose stage may want to lift R-skills-1…8, the hash_bundle algorithm, the public API surface, and the smoke matrix from `.codeless/jobs/starter-skills/SCOPE.md` into SKILLS.md verbatim so SKILLS.md actually fulfils the "source of truth" role.
- `DOCS/agent/SCOPE.md` was NOT touched (S-D5 explicitly keeps R4 as-is; per-job SCOPE and WORKFLOW both forbid editing it in this job).
- The per-job SCOPE's "Open questions" section uses bias notes (out-of-scope, file:// only, best-effort, keep R4) which I adopted as the decisions; no overrides were applied.
- No code changed, no tests run (prose-only stage). `cargo check` etc. not invoked.

## Open questions

- Should this session also push the commit to origin (`codeless/starter-skills`)? WORKFLOW.md closing-trio says yes for every stage, but the stage prompt only said "Commit your work". Left unpushed for the next session to decide.
- Whether a follow-on prose stage should lift R-skills-1…8 and the full normative body from `.codeless/jobs/starter-skills/SCOPE.md` into `DOCS/agent/SKILLS.md` so the file actually carries the "source of truth" role its header claims, or whether keeping the body in per-job SCOPE and pointing to it is acceptable indefinitely.
