# Workflow — rubix-thin-slice (v2)

How to drive the stages in `template.yaml`. Read this before
every stage alongside `SCOPE.md`, the authoritative source SCOPE
at
[`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md),
and the latest handoff at
[`/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-7.md`](/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-7.md).

## Sequencing

Three work stages, three REVIEW gates between them. Strictly
linear; no parallel stages.

```
stage 1 (block 1, PR 3 MCP)   ── REVIEW ──
stage 2 (block 2, PR 4 CH)    ── REVIEW ──
stage 3 (block 3, PR 5 REST + CLI)  ── REVIEW
```

Block 1 unblocks Block 2 (the alert-dispatch in PR 4 reuses the
same `Tool` registry composition mounted for MCP). Block 2
unblocks Block 3 (the REST handler's `?render=server` path
exercises the same `MessageBundle::render_diagnostic` call site
the CLI uses).

A REVIEW gate exists between every pair because each block adds
a load-bearing concern that, if wrong, contaminates the next:

- **After block 1** — a wrong locale plumbing (someone re-routes
  around U1's task-local instead of through it) silently breaks
  every later i18n assertion.
- **After block 2** — a direct `clickhouse` crate dep slipped
  into a rubix Cargo.toml is hard to spot once it propagates;
  a missing `tenant_id` on writes silently breaks future
  multi-tenant work.
- **After block 3** — a fat REST handler leaking domain logic
  invalidates the gRPC-swap smoke test forever.

## Per-stage discipline

Before writing any code in a stage:

1. **Re-read the corresponding block in the source SCOPE.** The
   stage text in `template.yaml` is the contract; this `WORKFLOW`
   is the process.
2. **Re-read `SCOPE.md` §"In scope", §"Out of scope", §"What is
   already landed", and §"What is already answered".** The
   biggest risk on this job is re-doing work that already
   happened ($2.65 was burned that way already). Master at
   `ad393ba` is the truth.
3. **Re-read `SCOPE.md` §"What is already answered".** T1, T2,
   T3, T4, T5, Q6, and the MCP-locale + `FlowAsTool` decisions
   are LOCKED. Do not re-litigate. If the stage text contradicts
   an answer there, the answer there wins.
4. **Run `cargo check --workspace` from the workspace root before
   any edit** so the baseline is known-clean.
5. **For block 1 (PR 3, MCP)**: run `git show 4c15dcb 9ab273d 7216d78`
   in the worktree so the U1/U2/U3 API shapes are in context.
   The wiring is essentially one line —
   `FlowAsTool::from_registry(&registry, &flow_id, &rev, engine).await?` —
   if you find yourself writing more, something has slipped.
6. **For block 2 (PR 4, CH)**: `grep -r '^clickhouse =' starter/crates/*/Cargo.toml`
   to see where it lives upstream (only `starter-store-clickhouse`).
   Confirm `cargo tree -p rubix-agent --invert clickhouse` shows
   only the transitive path before adding any CH code.
7. **For block 3 (PR 5, REST + CLI)**: `wc -l` your REST handler
   file before opening the PR. Over 20 lines means domain logic
   leaked — push back to `probe()`.

## During a stage

- **One file per verb. ≤400 lines hard, ~100 typical.** If a
  file approaches 300, stop and split.
- **Doc-tier rule.** Code comments reference
  `docs/design/<area>/README.md` only. `./rubix/scripts/lint-doc-refs.sh`
  enforces this — run it before closing a stage. Forbidden
  patterns to grep for:
  ```
  SCOPE\.md | HOW-TO-CODE\.md | NEW-SESSION\.md | FILE-LAYOUT\.md
  docs/scope/ | docs/sessions/
  ```
- **No phasing markers.** No `// Phase 0`, `// STAGE-1 done`,
  `// FIXED:`, `// Previously this used X`. The lint does not
  catch these; review must.
- **TODOs carry an owner or upstream tag.** `// TODO(name): ...`
  or `// TODO(upstream: <issue>): ...`. Never bare TODOs.
- **Block 2's hardcoded insights check uses the EXACT comment
  text** named in stage 2: `// TODO(upstream: rule.rhai migration) — promote to starter-insights::RuleRegistry once a second rule appears.`
  The reviewer greps for this string verbatim.

## When stuck

Codeless cannot ask the human. The escape hatch:

1. Stop work on the current block immediately.
2. Open the PR anyway with whatever compiles.
3. Add `BLOCKED: <one-line question>` to the PR description plus
   a paragraph explaining what was tried.
4. Move to the next block only if it does not depend on the
   blocked one. Otherwise stop and wait.

The human reviews the blocked PR and answers. Codeless does not
guess to unblock itself.

## At a REVIEW gate

The human runs through the REVIEW stage's checklist literally.
Each item is a one-line shell command, grep, or file read. If any
item fails, the previous stage's PR is amended (NOT replaced —
the next stage stays paused) until the checklist is green.

A REVIEW that passes triggers the next stage with the same
discipline.

## When the job is finished

When stage 3 lands and its REVIEW gate passes, **codeless stops**.
The job is NOT finished until the human runs the six-step manual
smoke in
[`rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
§"Success criterion":

```bash
mani run run                                  # boot
curl ... /api/v1/auth/login                   # log in as bootstrap operator
curl -H "Accept-Language: es-AR" ...          # call disk via REST
# claude desktop: open MCP, call the flow
psql -c "SELECT * FROM changelog ..."         # inspect audit
clickhouse-client -q "SELECT * FROM ..."      # inspect history
```

If any step fails, the human files a one-line issue per failure;
codeless takes a follow-up job to fix.

## Anti-patterns specific to this job

- **Re-implementing U1/U2/U3.** They are landed; read the
  commits, don't reinvent.
- **Hand-rolling `FlowAsTool`.** `FlowAsTool::from_registry` is
  the contract.
- **Adding a direct `clickhouse` crate dep to rubix.** Pull
  transitively only.
- **Translating skill bodies or tool descriptors.** EN canonical.
- **Touching `rubix-client`.** Q6 deferred.
- **Removing the dev-dep pin in `starter-mcp/Cargo.toml:37-42`.**
  Phase 2c fix is a separate job.
- **Re-doing Phase 2a Postgres stores.** Already landed (commit
  `51e3ed8`). Touching them creates conflicts.
- **Re-doing Path B bootstrap-user.** Already landed (commit
  `5083d87`).

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's `verify:` list (or `verify_cmd`).
   Every step must pass. On failure: stop, fix, re-run; do not
   advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the active
   session doc, in the same worktree, so the fresh agent that opens
   the next stage has the context it needs (per SCOPE Constraint 2:
   anything that must survive a stage boundary is on disk, not in
   the agent's head).
3. `git` — stage the changes (`git add -A` from the worktree root,
   or specific paths if the stage was surgical), commit with the
   message `stage N: <one-line title from template.yaml>` so the
   history mirrors the template stages one-for-one, and push to
   the job's branch (`codeless/rubix-thin-slice-v2`) so the work
   is recoverable even if the worktree is wiped.

A stage is not "done" until all three todos are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry — do
not mark the stage `[x]`, do not advance, and never `--force` or
`--no-verify`. If a stage genuinely produced no change (e.g. an
investigation stage that only updated `SCOPE.md` and that doc was
already current), say so in the handover and mark `git` as
`skipped — no diff`, but the next stage's commit must include any
side-effect files the investigation touched.

## References

- Source SCOPE:
  [`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
- Current handoff:
  [`/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-7.md`](/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-7.md)
- Per-job scope:
  `./SCOPE.md`
- Rubix architecture:
  [`/home/user/code/rust/starter/rubix/SCOPE.md`](/home/user/code/rust/starter/rubix/SCOPE.md)
- Contributor entry point:
  [`/home/user/code/rust/starter/rubix/HOW-TO-CODE.md`](/home/user/code/rust/starter/rubix/HOW-TO-CODE.md)
- File-layout rules:
  [`/home/user/code/rust/starter/rubix/FILE-LAYOUT.md`](/home/user/code/rust/starter/rubix/FILE-LAYOUT.md)
- Session boot:
  [`/home/user/code/rust/starter/rubix/NEW-SESSION.md`](/home/user/code/rust/starter/rubix/NEW-SESSION.md)
- Upstream PR ledger:
  [`/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md`](/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md)
