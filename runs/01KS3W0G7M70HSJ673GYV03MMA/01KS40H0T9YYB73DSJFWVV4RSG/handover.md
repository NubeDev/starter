## Done

- Added `scripts/check-skills-dep-isolation.sh`: runs `cargo tree -p starter-skills --edges normal` and fails on any of the seven banned provider-SDK crate names (async-openai, anthropic-ai-sdk, anthropic-sdk, google-genai, aws-sdk-bedrockruntime, mistralai, ollama-rs). Anchored "^<crate> v[0-9]" to avoid substring false positives. Locally returns "no banned provider SDK in normal-deps closure."
- Wired the gate into CI as a new `skills-dep-isolation` job in `.github/workflows/ci.yml`, mirroring the existing `spi-dep-baseline` job shape (checkout + toolchain + rust-cache + script). The job is PR-blocking, not an after-the-fact audit.
- Added an explicit `cargo test -p starter-flow-nodes --features ai-agent --no-fail-fast` step to the rust CI job so smoke #7's node-side half (`stage7_ai_agent_mount.rs::resource_hash_mismatch_aborts_the_run_then_reload_proceeds`) actually runs — `ai-agent` is default-off so default `cargo test --workspace` skipped it.
- Appended a "Smoke matrix (CI-pinned)" section to `DOCS/agent/SKILLS.md` mapping each of the nine normative smokes to its concrete test path, plus a "Dep-tree isolation gate" section documenting the banned-crate list + how to change it.
- Verified: all 44 starter-skills tests pass; `cargo test -p starter-flow-nodes --features ai-agent --test stage7_ai_agent_mount` passes; dep-isolation script exits 0 against the current tree.

## Next

- (none) — Stage 12 is the final stage of the starter-skills job per the WORKFLOW. A fresh session may pick up the follow-on `starter-cli` job referenced by S-D1 (approval CLI surface).

## What you need to know

- All nine smokes were already pinned by tests landed in earlier stages (Phase 2 inline tests for #1/#8/#9, Phase 3 registry tests for #2/#3, Phase 4 selector tests for #4/#5, Phase 4b mount tests for #6/#7). Stage 12 added zero new tests — its deliverable was the CI wiring + the dep-tree gate.
- Rows 1 and 9 of the smoke matrix share `tests::line_ending_normalisation_is_stable`. The per-job SCOPE enumerates them separately, but the CRLF→LF / lone-CR→LF transform is one invariant; the matrix table flags this in a follow-on note.
- The dep-isolation script matches the start-of-line normalized form `^<crate> v[0-9]`. Adding a banned crate is one line in the script's `BANNED=(...)` array + an update to the docs paragraph; removing one is explicitly a SCOPE-level conversation (R2 isolation).
- Commit: `2da5e5a` on branch `codeless/starter-skills`. Stage 12 commit message starts with the full stage title as required.

## Open questions

- (none)
