# Workflow — rubix-thin-slice

How to drive the stages in `template.yaml`. Read this before
every stage alongside `SCOPE.md` and the authoritative source
SCOPE at
[`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md).

## Sequencing

Five work stages, four REVIEW gates between them. Strictly
linear; no parallel stages.

```
stage 1 (block 1) ── REVIEW ── stage 2 (block 2) ── REVIEW ──
stage 3 (block 3) ── REVIEW ── stage 4 (block 4) ── REVIEW ──
stage 5 (block 5)
```

Block 1 is upstream in `starter/` only — no rubix code touched.
Blocks 2–5 are rubix-side and each consumes the previous block's
seams:

- **Block 2** consumes Block 1's `PgSessionStore` / `PgTokenStore`
  / `PgTenantStore` for cookie sessions + tenant lookup.
- **Block 3** consumes Block 2's authz gate (MCP calls go through
  the same gate as REST).
- **Block 4** consumes Block 3's tool-registry composition (the
  ClickHouse-write hook lives in the same place tools are
  dispatched).
- **Block 5** consumes Block 4's full tool surface (REST and CLI
  hit the same `probe()` plus the ClickHouse-write hook plus the
  alert-on-threshold logic).

A REVIEW gate exists between every pair because every block adds
a load-bearing concern that, if wrong, contaminates the next
block:

- After block 1 — wrong error-code mapping in the Postgres ports
  silently breaks every auth path in block 2.
- After block 2 — a non-idempotent `bootstrap_user` makes every
  subsequent test setup brittle; a missing authz gate makes the
  MCP block silently public.
- After block 3 — an FlowAsTool slip means block 5 has to wire
  REST per-flow too, doubling the surface.
- After block 4 — a misplaced direct `clickhouse` crate dep on
  rubix is hard to spot later once it's transitively pulled in
  by half the workspace.

## Per-stage discipline

Before writing any code in a stage:

1. **Re-read the corresponding block in the source SCOPE.** The
   stage text in `template.yaml` is the contract; this `WORKFLOW`
   is the process.
2. **Re-read `SCOPE.md` §"In scope" and §"Out of scope".** The
   biggest risk on this job is silent scope creep — the source
   SCOPE explicitly carves out tempting features (gRPC,
   `rubix-client` wiring, OAuth, dashboards, cron, `rule.rhai`).
   Stay within the carve-outs.
3. **Re-read `SCOPE.md` §"What is already answered".** T1, T2,
   T3, T4, T5, Q6, and the MCP-locale decision are LOCKED. Do
   not re-litigate. If the stage text contradicts an answer
   there, the answer there wins.
4. **Run `cargo check --workspace` from the workspace root before
   any edit** so the baseline is known-clean.
5. **For block 1**, `grep -rn 'SqliteSessionStore\|SqliteTokenStore\|SqliteTenantStore' starter/crates/starter-auth-users/src/` to enumerate every site the ports need to mirror; confirm the count matches what the source SCOPE expects. If the count is wrong, surface before editing.
6. **For block 2**, `psql -c '\d+' against the target DB` to
   confirm starter migrations already created the user/session/
   token tables before rubix 0001_init runs.
7. **For block 3**, run the `mcp_disk_test` skeleton once before
   adding assertions to confirm the MCP harness from
   `starter-server::testing` actually wires up against the rubix
   binary.
8. **For block 4**, confirm `starter-store-clickhouse` is on the
   `Cargo.toml` path **only via the rubix-agent transitive
   path** — `cargo tree -p rubix-agent --invert clickhouse`
   should show only `starter-store-clickhouse` in the chain.
9. **For block 5**, run `wc -l` on the candidate REST handler
   file before opening the PR. Over 20 lines means domain logic
   leaked — push back to `probe()`.

## During a stage

- **One file per verb. ≤400 lines hard, ~100 typical.** If a
  file approaches 300, stop and split.
- **Doc-tier rule.** Code comments reference
  `docs/design/<area>/README.md` only. `mani run lint-doc-refs`
  enforces this — the codeless agent does NOT need to run mani,
  but a fresh grep against the forbidden patterns must return
  clean before the stage closes. The forbidden patterns:
  ```
  SCOPE\.md | HOW-TO-CODE\.md | NEW-SESSION\.md | FILE-LAYOUT\.md
  docs/scope/ | docs/sessions/
  ```
- **No phasing markers.** No `// Phase 0`, `// STAGE-1 done`,
  `// FIXED:`, `// Previously this used X`. The lint does not
  catch these; review must.
- **TODOs carry an owner or upstream tag.** `// TODO(name): ...`
  or `// TODO(upstream: <issue>): ...`. Never bare TODOs.

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

When stage 5 lands and its REVIEW gate passes, **codeless stops**.
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

## References

- Source SCOPE (authoritative):
  [`/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md`](/home/user/code/rust/starter/rubix/docs/scope/THIN-SLICE.md)
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
- Latest session handoff:
  `/home/user/code/rust/starter/rubix/docs/sessions/2026-05-23-next-steps-6.md`
- Forward gaps:
  [`/home/user/code/rust/starter/rubix/docs/scope/GAPS.md`](/home/user/code/rust/starter/rubix/docs/scope/GAPS.md)
- Upstream PR ledger:
  [`/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md`](/home/user/code/rust/starter/rubix/docs/design/starter-changes/README.md)

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
   the job's branch (`codeless/rubix-thin-slice`) so the work is
   recoverable even if the worktree is wiped.

A stage is not "done" until all three todos are green and the push
succeeds. If `checks` or `git` fails, fix the cause and retry — do
not mark the stage `[x]`, do not advance, and never `--force` or
`--no-verify`. If a stage genuinely produced no change (e.g. an
investigation stage that only updated `SCOPE.md` and that doc was
already current), say so in the handover and mark `git` as
`skipped — no diff`, but the next stage's commit must include any
side-effect files the investigation touched.
