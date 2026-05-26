# Scope — puck-builder-finish

Authoritative design lives at:

- [/home/user/code/rust/starter/rubix/docs/scope/dashboards/10-puck-builder.md](/home/user/code/rust/starter/rubix/docs/scope/dashboards/10-puck-builder.md) — Puck builder scope (sections §B1–§B6).
- [/home/user/code/rust/starter/rubix/docs/scope/dashboards/11-live-canvas-sse.md](/home/user/code/rust/starter/rubix/docs/scope/dashboards/11-live-canvas-sse.md) — live-canvas SSE banner.

Where this brief disagrees with either scope doc, **the scope docs
win** — fix this file rather than diverge. Neither scope doc may be
edited by this job.

## Goal

Finish the Puck visual SDUI editor begun in commit `bd8eeab`. The
editor already loads at `/dashboards/$pageId/edit`, edits the IR
tree, saves through `rubix.dashboard.update`, and surfaces 409
conflicts in a modal — that work is §B1, §B2, §B4, §B5, and the
§B6 CI drift guard. This job lands the remaining items:

1. **§B3 data-source selectors** — replace the text-field fallback
   for every `$ref`-typed IR leaf with a catalogue-backed
   select/autocomplete.
2. **§B6 runtime schema-hash banner** — runtime drift detection
   between the agent's IR schema and the frontend's committed copy.
3. **Scope 11 — live-canvas SSE** — banner + revalidate when
   someone else (or the AI) saves the same page while the operator
   is editing.
4. **Follow-ups from §B4 / §B5 work**:
   - Real tenant wiring (drop hardcoded `"system"`).
   - Ref-based discard bridge (drop the `window.__rubixPuckDiscardRequested`
     polling shim).
   - Placeholder coverage expansion for every IR variant currently
     falling through to the dangling tile.
   - Fix the stale `"3 series"` assertion in
     `packages/starter-ui-sdui-react/src/renderer/__tests__/render-chart.test.tsx`.

After this job:

- `@nube/starter-ui-sdui-puck` has full feature parity with the
  read-side renderer (`@nube/starter-ui-sdui-react`).
- The README status table is all ✅, no ⏳ rows.
- Manual two-tab verify works: edit in tab A, save from agent CLI
  or tab B, tab A shows the live-canvas banner without a refresh.

## In scope (five stages, no REVIEW gates)

- **Stage 1 — §B3 data-source selectors.** Catalogue seam in
  `@nube/starter-ui-sdui-puck` (rubix-agnostic, like the save seam),
  rubix-frontend fills it from the live `/api/v1/tools` catalogue.
- **Stage 2 — tenant + discard-bridge + render-chart cleanup.**
  Real session tenant, ref-based discard, one-line test fix.
- **Stage 3 — placeholder coverage.** Per-variant fillers in
  `@nube/starter-ui-sdui-react/src/headless/placeholder-render.tsx`
  for every variant currently falling through.
- **Stage 4 — §B6 runtime schema-hash banner.** Build-time hash of
  the committed schema; runtime fetch from the agent; banner on
  divergence.
- **Stage 5 — Scope 11 live-canvas SSE.** Banner + auto-revalidate
  in `PuckBuilder` when a `dashboard.updated` SSE event arrives for
  the page being edited; reuses the §B4 conflict-modal mechanics.

## Out of scope

- **Editing the scope docs.** `10-puck-builder.md` and
  `11-live-canvas-sse.md` are locked specs. Update the package
  READMEs instead.
- **Touching `crates/` or `rubix/crates/`.** The Rust side is
  frozen for this job. If the agent thinks a Rust change is
  required (e.g. a missing verb), halt and surface — do not edit.
- **Adding new top-level npm dependencies without surfacing.** Minor
  version bumps to existing deps are fine (Puck went 0.18 → 0.19
  pre-job); brand-new packages need to be flagged in the handover.
- **Editing `pnpm-workspace.yaml`.** Unchanged.
- **Designing for hypothetical future requirements.** No second
  catalogue source, no "for-future" abstractions, no half-finished
  scaffolding.
- **Replacing the Puck dependency.** 0.19 is the target.
- **Refactoring the IR or the read-side renderers beyond what
  scope 11 forces** (the SSE hook may need to be extracted from
  `sdui-page.tsx` into a shared `useDashboardEvents` if scope 11
  requires it — that's allowed; broader cleanup is not).
- **MCP-only refactor of the AI surface.** Tracked separately per
  the user's long-term direction.

## Constraints

- **Do not touch `crates/` or `rubix/crates/`.** Frozen for this
  job. The agent operates in `packages/`, `rubix/packages/`, and
  `rubix/frontend/` only.
- **Do not edit either scope doc.** Status flows into package
  READMEs.
- **Do not edit `pnpm-workspace.yaml`.**
- **Stop and surface before adding a new top-level npm
  dependency.** A halt, not a guess.
- **Tests live with the code.** Every new field type, new
  placeholder, new hook, new banner gets a vitest test in the
  same package.
- **No `--force`, no `--no-verify`.** Fix the hook, not the
  command.
- **Per-stage commit + push to `codeless/puck-builder-finish`** —
  one stage, one commit, see WORKFLOW.md "Closing trio".
- **R12** — comments explain why, never what. No graveyard
  markers ("removed in stage 2" etc); the commit message
  carries that.

## Deliverables (what "done" looks like)

1. `codeless/puck-builder-finish` branch with one commit per
   stage (five stages = five commits), pushed.
2. `pnpm --filter @nube/starter-ui-sdui-puck typecheck` +
   `test` green at every stage boundary.
3. `pnpm --filter @nube/starter-ui-sdui-react test` green at
   every stage boundary.
4. `pnpm --filter @nube/rubix-frontend typecheck` green at every
   stage boundary.
5. `packages/starter-ui-sdui-puck/README.md` status table has no
   ⏳ rows; every scope row is ✅; "Next tasks" section is gone
   or replaced with a note that the scope is fully implemented.
6. Manual two-tab verify against `/dashboards/data-flow-site-a/edit`:
   - $ref leaves render as catalogue-backed selects (stage 1).
   - No `"system"` literal in route or hook (stage 2).
   - Discard reloads synchronously, no 250ms tick (stage 2).
   - Every IR variant drops onto the canvas with a visible
     placeholder, never the dangling-variant tile (stage 3).
   - Forcing a schema mismatch (mutate the committed schema
     locally) surfaces the runtime banner (stage 4).
   - Saving from a second tab or the agent CLI surfaces the
     live-canvas banner in the editing tab (stage 5).
7. The `verify` block from WORKFLOW.md ran clean for every
   stage and the transcript is in the handover.

## Open questions — RESOLVED (2026-05-26, before start)

### Q1 — Single job or split per stage?

**Answer: single job, five stages, no REVIEW gates.**

User explicitly asked for "one big job, do everything, codeless
can run for hours". Stages are loosely coupled but the scope is
small enough to land together. If a stage genuinely surprises
(see WORKFLOW.md "When to halt"), the agent halts rather than
powering through.

### Q2 — Catalogue verb names for §B3?

**Answer: discover at stage warmup, do not guess.**

The job runs against a live `rubix-agent` (the user's setup boots
it on `127.0.0.1:8088`). Stage 1 starts by listing
`/api/v1/tools` to find the analytics-template / tool / tenant
catalogue verbs by name. Halt if any required verb is missing —
that is a Rust-side change which is out of scope for this job.

### Q3 — Tenant source for stage 2?

**Answer: discover the existing session hook, do not invent.**

`packages/starter-client-react` already exposes the authed
session (likely `useSession` or `useMe`). Stage 2 starts by
reading that package's exports. If no tenant field is on the
session shape, halt — that is a Rust-side change.

### Q4 — Does scope 11 mandate the existing SSE plumbing?

**Answer: yes, reuse it.**

`packages/starter-ui-sdui-react/src/headless/sdui-page.tsx`
already subscribes to per-page events for the read path. Stage 5
either reuses that subscription or extracts the SSE machinery
into a shared `useDashboardEvents` hook. Do not stand up a
parallel SSE client.

## References

- Scope docs (authoritative — do not edit):
  - `rubix/docs/scope/dashboards/10-puck-builder.md`
  - `rubix/docs/scope/dashboards/11-live-canvas-sse.md`
- Package READMEs (this job updates):
  - `packages/starter-ui-sdui-puck/README.md`
  - `packages/starter-ui-sdui-react/README.md` (if §B2 follow-up
    note in stage 3 lives here too)
- Save seam pattern (precedent for the §B3 catalogue seam):
  - `packages/starter-ui-sdui-puck/src/save.ts`
- 409 modal pattern (precedent for the scope 11 banner):
  - `packages/starter-ui-sdui-puck/src/builder.tsx`
- Existing edit route (the integration point):
  - `rubix/frontend/src/routes/dashboards/$pageId_.edit.tsx`
- Pre-job memory (read it):
  - `~/.claude/projects/-home-user-code-rust-starter/memory/MEMORY.md`
