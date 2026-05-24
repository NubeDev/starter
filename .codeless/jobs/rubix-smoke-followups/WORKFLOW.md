# Workflow — rubix-smoke-followups

## Sequencing

Eight stages. Three review-and-commit of already-implemented work (1–3). One REVIEW gate that decides PR shape (4). Three new implementation stages (5–7). One closing REVIEW + final docs (8).

Stages 1 → 2 → 3 must run in that order because Group A's starter-flow change is depended on by rubix-agent's narration-on default, and Group B's observability change is depended on by Group C's port pre-flight log message clarity. Stages 5, 6, 7 are independent of each other and can run in any order after stage 4's REVIEW.

## Per-stage discipline

### Stages 1–3 — review and commit already-implemented work

You are **not re-implementing**. The work is on disk in the working tree. Your job:

1. **Read the relevant section of the smoke session note first** — `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md`. The §"Re-run after fixes" table maps bug ID → fix → file; the §"Engine-coordinator quiescence" subsection covers Group A's starter-flow piece in depth; the §"Files changed" list at the bottom is the canonical inventory.
2. **Run `git diff <files-for-this-group>` and read every hunk.** For each hunk, ask: does the change match the narrative? Does it follow R1 (verb-per-file, ≤ 400 lines)? Does it follow R3 (no `SCOPE.md` / `docs/scope/` / `docs/sessions/` link in code comments)? If yes, it's good. If no, raise the deviation in the handover with a one-paragraph justification of what you'd change, then apply the change or raise BLOCKED for the operator's call.
3. **Run the stage's verify commands.** Stage 1: `cargo test -p starter-flow -p starter-ai-agent -p rubix-agent`. Stage 2: `cargo test -p starter-observability -p starter-mcp`. Stage 3: `cargo test -p rubix-agent` plus a manual `mani run demo` smoke (boot until you see `mcp_tools=6` and `migrations_skipped=false`; then Ctrl-C).
4. **`./rubix/scripts/lint-doc-refs.sh` clean.** Non-negotiable, every stage.
5. **Commit per the stage description.** Group A is three commits in chronological dependency order (starter-flow → starter-ai-agent → rubix-agent) so each one builds standalone; group B is one commit; group C is one commit. Use a HEREDOC for the body so formatting holds.
6. **Push to `codeless/rubix-smoke-followups`.** No PR yet — that's stage 4's REVIEW.

If `cargo test` is red on a file that's part of your stage, **the diff has a real defect.** Fix it on top of the existing diff with a small additional commit (`fix: <one-line>`) that the next stage's review covers. Do not revert and re-implement.

### Stage 4 — REVIEW gate, PR shape decision

Pause. In `handover.md` for the next stage, lay out:

- Each of the five commits made in stages 1–3, with its one-line title and the bug(s) it closes.
- The `cargo test` summary per stage (counts only — no diff dumps).
- The two PR shape options from SCOPE Open Question 4, with your recommendation (default: one PR with stacked-commit history reviewed commit-by-commit).
- The proposed PR title, body, and base / head for whichever shape the operator confirms.

When the operator confirms, run `gh pr create --base master --head codeless/rubix-smoke-followups` with the agreed title and body. Push happened in stages 1–3; the `create` call is read-only against the remote.

Stage 4 commits nothing of its own. Its `git` closing-todo reads `skipped — gate-only`.

### Stages 5–7 — new implementation

Standard implementation discipline:

1. **Read the relevant SCOPE Open Question first.** Stage 5 = OQ-2 (CH database routing). Stage 7 = OQ-3 (alert-path test).
2. **Read the existing code with `git log -p` + `grep`** to ground the design choice. Stage 5: `grep -rn 'CREATE DATABASE rubix\|USE rubix' crates/starter-store-clickhouse rubix/`. Stage 6: `grep -rn 'SUPER_ADMIN_TENANT' crates/` and `git log -G SUPER_ADMIN_TENANT`. Stage 7: read `rubix/crates/rubix-agent/src/boot/insights.rs` for the hardcoded 90.
3. **Pick the smaller move per SCOPE.** If genuinely > 100 LoC of refactor, raise BLOCKED with the LoC estimate and a one-paragraph design sketch of the larger move.
4. **Test lives with the code in the same commit.**
5. **Catalogue files updated in the same commit** for any new MessageKey.
6. **`cargo test`** for the touched crate(s) green. **`./rubix/scripts/lint-doc-refs.sh`** clean. Commit, push.

### Stage 8 — closing REVIEW

Write the handover summarising stages 5–7 commits. Confirm with operator that the follow-up PR shape should be one PR off the same branch (default). On confirmation, run `gh pr create`. Then update the smoke session note with a "Follow-ups landed" subsection citing each PR by number. If the alert path now exercises, flip the THIN-SLICE success-criterion row to verified-on-<today>. Commit, push, open the final docs PR.

## Anti-patterns specific to this job

- **Don't re-derive the implementation in stages 1–3.** The work is on disk, tested, documented. Your job is review-commit-push.
- **Don't squash the three Group A commits into one.** The dependency order (starter-flow → starter-ai-agent → rubix-agent) is the whole point — each commit must build standalone so `git bisect` works for any future regression.
- **Don't open any PR before stage 4's REVIEW.** The operator chooses the PR shape.
- **Don't fix B9 / B10 / N4 / alert in stages 1–3.** Those are stages 5–7. Keep stages 1–3 about the already-implemented work only.
- **Don't list paths with brace expansion in the handover.** `routes/{mod.rs,tools.rs}` will trip the diff-verify pre-check. List every path individually.
- **Don't list a path under "Done" that the stage didn't modify.** Same diff-verify trap. "Done" means "files I modified," not "files I touched / scripts I executed."
- **Don't `--no-verify`, don't `--force`** (only `--force-with-lease` after explicit rebase need, with operator confirmation). If a hook fails, fix the cause.

## REVIEW gate behaviour

A REVIEW stage still **commits and pushes the stage that led to it**. The REVIEW is for the *next* stage. In this job stages 4 and 8 are REVIEW gates: stage 4 commits nothing of its own (gate-only); stage 8 commits the closing docs.

When a REVIEW gate fires, write the gate's question into `handover.md` for the next stage, then halt. The operator opens the job UI, reads the handover, and either approves to continue or asks a follow-up. Do not advance past a REVIEW without explicit operator confirmation.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list (or verify_cmd). Every step must pass. On failure: stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active session doc, in the same worktree, so the fresh agent that opens the next stage has the context it needs (per SCOPE Constraint 2: anything that must survive a stage boundary is on disk, not in the agent's head).
3. `git` — stage the changes (`git add -A` from the worktree root, or specific paths if the stage was surgical), commit with the message `stage N: <one-line title from template.yaml>` so the history mirrors the template stages one-for-one, and push to the job's branch (`codeless/rubix-smoke-followups`) so the work is recoverable even if the worktree is wiped.

A stage is not "done" until all three are green and the push succeeds. Never `--force`, never `--no-verify`. If a stage produced no change (gate-only stage 4), say so in the handover and mark `git` as `skipped — gate-only`.

**Important caveat for this job:** stages 1–3 each contain **multiple commits** (Group A is three commits, B and C are one each). The closing-trio `git` step in stages 1–3 means "all this stage's commits are made and pushed." For single-commit stages it reads as in any other job.

## Hard rules (repeated)

- One verb per file. ≤ 400 lines hard, ~100 typical. No `utils.rs` / `helpers.rs` / `common.rs` / `misc.rs`.
- Code comments link `docs/design/<area>/README.md` only. Never `SCOPE.md`, `HOW-TO-CODE.md`, `NEW-SESSION.md`, `FILE-LAYOUT.md`, `docs/scope/`, or `docs/sessions/`. `./rubix/scripts/lint-doc-refs.sh` enforces it.
- No phasing markers in code. No `// Phase 0`, `// STAGE-1 done`, `// FIXED:`.
- Upstream-first. starter-flow / starter-ai-agent / starter-mcp / starter-observability land before rubix consumes them. Stage ordering respects this.
- Tool outputs are `Diagnostic` + structured data, never pre-formatted strings.
- Catalogue files are the source of truth for MessageKeys. No new key without entries in both `en.json` and `es.json` in the same commit.
- Tests live with the code in the same commit.
- Comments explain *why*, not *what*. No emojis.

## References

- `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md`
- `rubix/docs/sessions/2026-05-24-handover-codeless-orchestration.md`
- `rubix/SCOPE.md`
- `rubix/FILE-LAYOUT.md`
- `rubix/HOW-TO-CODE.md`
- `rubix/NEW-SESSION.md`
