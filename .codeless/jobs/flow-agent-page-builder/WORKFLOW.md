# Workflow — flow-agent-page-builder

How to drive this job. The shape is "pin four small design points
first, then land the slice in two layers (lib → pages) and wire
the shell + acceptance pass last."

## Sequencing

- Stage 1 is **prose-only**. Pin the four design points in
  [SCOPE.md](./SCOPE.md), record under "Decisions", commit. No
  code.
- Stage 3 (lib layer) lands first because every route screen
  imports from it. Land it as one commit so the API surface for
  `pages-store`, `builder-fixture`, and `<SduiHost>` is stable
  before the pages reference it.
- Stage 4 (route screens) lands on top of the lib layer.
  `PageBuilder.tsx` is the most subtle — compose `useBuilder` +
  `<BuilderTranscript>` + `<AiBuilderCanvas>` directly per R1; do
  not wrap `<AiBuilder>`.
- Stage 5 (routes + shell nav + acceptance) is the merge gate.
  No partial landings; the eight acceptance checks all pass
  before the stage commits.

## Per-stage discipline

- Before any code change in a stage:
  - `git log -20 --oneline` for the surrounding history.
  - Re-read [SCOPE.md](./SCOPE.md) "Hard rules". R1 (compose,
    don't wrap), R2 (`useSyncExternalStore` for sidebar), R3
    (pinned fixture timings), R4 (`<SduiHost>` is a no-op
    `SduiProvider`), R5 (no `package.json` edits) are
    load-bearing.
  - Re-read the relevant lib's source before importing from it:
    - `packages/starter-ui-ai-builder/src/hooks/use-builder.ts`
      for the buffer window and phase machine.
    - `packages/starter-ui-ai-builder/src/components/ai-builder.tsx`
      for the canvas/transcript composition the host mirrors.
    - `packages/starter-sdui-react/src/types.ts` for
      `UiComponentTree.root: UiComponent`.
    - `packages/starter-ui-skills/src/adapters/in-memory.ts` for
      the `createInMemorySkillsAdapter` mutation semantics.
- Touch only what the stage names. No drive-by refactors of the
  existing flow-agent shell beyond the route + nav additions.
- Verify before commit:
  - **Typecheck**: `pnpm --filter flow-agent-frontend typecheck`.
  - **Build**: `pnpm --filter flow-agent-frontend build` for any
    stage that touches route screens or shell wiring.
  - **Manual smoke** (stage 5 only): `pnpm --filter
    flow-agent-frontend dev`, walk the eight acceptance checks
    from [PAGE-BUILDER.md](../../../examples/flow-agent/PAGE-BUILDER.md)
    end-to-end, capture any console errors. Then run
    `pnpm --filter flow-agent-frontend build && pnpm --filter
    flow-agent-frontend exec vite preview` and re-walk
    `/pages/:id` for console errors / layout-shift warnings.
- Commit only if green. One logical batch per commit; commit
  message stage-tagged: `stage N: <one-line title>`.

## REVIEW gates

One:

- **After stage 1** — decisions sign-off before any code lands.
  The four design points (sidebar live-update mechanism, fixture
  timings, `<SduiHost>` shape, package.json invariant) carve out
  the slice; locking them down first is cheap.

Write a one-line summary into the handover at the gate. Do not
proceed.

## What "done" looks like per stage

| Stage | Done when |
|---|---|
| 1 | SCOPE.md "Decisions" section filled in for the four design points; no code changed. |
| 3 | `src/lib/pages-store.ts` exposes typed CRUD + `subscribe(fn)` + a `usePages()` `useSyncExternalStore` hook listening to both in-process subscribers and `window.storage`; `src/lib/builder-fixture.ts` exports a `createFlowAgentBuilderFixture()` returning a `BuilderAdapter` with pinned timings and the five prompt prefixes; `src/lib/sdui-shim.tsx` exports `<SduiHost>` wrapping `SduiProvider` with a no-op action dispatcher; `pnpm --filter flow-agent-frontend typecheck` clean. |
| 4 | All four route screens compile against the stage-3 lib layer; `PageBuilder.tsx` calls `useBuilder` directly (does **not** import `<AiBuilder>` — R1); the save button writes a fully-typed `{id, name, tree, createdAt}` record via `pages-store` and navigates to `/pages/:id`; `<PageView>` renders inside `<SduiHost>`; `<Skills>` mounts `<SkillsManager>` with `createInMemorySkillsAdapter` seeded from the two reference bundles; typecheck clean. |
| 5 | Five new routes wired in `app.tsx`; `Shell.tsx` Pages section subscribes via `usePages()` from stage 3 and renders saved pages live; Skills leaf links to `/skills`. The eight acceptance checks pass — empty state on first visit, `sales` prompt streams in <2s with buffered-patch badge briefly visible, save → `/pages/:id` no-CLS no-console-errors, Edit ⇄ View round-trip zero diff, sidebar live in-tab + cross-tab, Skills approve flips without manual reload, `pnpm --filter flow-agent-frontend typecheck` clean, `pnpm --filter flow-agent-frontend build` succeeds and `vite preview` of the build on `/pages/:id` reports no console errors / no layout-shift warnings. |

## Anti-patterns

- Wrapping `<AiBuilder>` in `PageBuilder.tsx`. R1 — the
  opinionated wrapper owns `useBuilder` internally and the host
  cannot reach `tree` to wire the save button. Compose
  `useBuilder` + `<BuilderTranscript>` + `<AiBuilderCanvas>`
  directly; the `ai-builder.tsx` source itself recommends this
  for full control.
- Reading `localStorage` directly from `Shell.tsx` and hoping
  React re-renders. R2 — pure reads do not trigger updates. Use
  `pages-store.subscribe(fn)` + `window.storage` via
  `useSyncExternalStore`.
- Hand-tuned `setTimeout` delays in the fixture without a recorded
  rationale. R3 — pin the timings in `builder-fixture.ts` with a
  comment referencing the `useBuilder` buffer window. A flaky
  buffered-patch demo is worse than no demo.
- A bespoke `<SduiHost>` that re-implements `SduiProvider`. R4 —
  the shim is a thin wrapper that installs a no-op action
  dispatcher; reuse the published provider.
- Adding `@nube/*` deps to `frontend/package.json`. R5 —
  everything used here is already a dep. A diff means the slice
  has strayed.
- Persisting pages to a server endpoint. Out of scope for v0.1;
  `localStorage` is the contract.
- Touching Rust crates or backend code. Out of scope; this slice
  is purely frontend and proves the libs compose before the
  backend lands.
- Shipping the theme-builder slice in this job. Reuses the same
  `BuilderEvent` shape but is a separate slice
  (`@nube/starter-ai-builder-react-theme`).
- Inserting drive-by refactors of the existing flow-agent shell
  (FlowsList, AgentsList, Settings, etc.) "while you're in
  there". Out of scope; touch only what the stage names.
- Wiring an SSE / fetch adapter "for later". Stage 5's adapter is
  scripted-only; swap-in is a follow-up when
  `starter-flow-node-ai-builder` lands.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in
order. The user watches these tick over in the `Stages` overview;
they are how the user confirms a long-running stage actually
landed instead of just looking like it did. Do **not** rename or
reorder them.

1. `checks` — run the stage's verify list (`pnpm --filter
   flow-agent-frontend typecheck` for every stage; `build` and
   the manual acceptance walk for stage 5). Every step must
   pass. On failure: stop, fix, re-run; do not advance to `docs`.
2. `docs` — update `handover.md` for the next stage and the
   active session doc, in the same worktree, so the fresh agent
   that opens the next stage has the context it needs.
3. `git` — stage the changes, commit with the message
   `stage N: <one-line title from template.yaml>`, and push to
   the job's branch (`codeless/flow-agent-page-builder`).

A stage is not "done" until all three are green and the push
succeeds. Never `--force`, never `--no-verify`; if a hook fails,
fix the cause.
