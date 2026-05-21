# Scope — flow-agent-page-builder

> Source of truth: [`examples/flow-agent/PAGE-BUILDER.md`](../../../examples/flow-agent/PAGE-BUILDER.md)
> in the starter repo. This file is the per-job brief the runner
> reads before every stage; it is intentionally short. When this
> file disagrees with `PAGE-BUILDER.md`, that doc wins — open an
> issue and update this file.

## Goal

Ship the **Page Builder slice** of `examples/flow-agent`: a
reference UX for AI-authored SDUI pages, layered onto the existing
React shell with **zero new Rust code**. Two new sidebar entries
(**Pages**, **Skills**) plus four new route screens prove the four
reusable React libs (`@nube/starter-ui-ai-builder`,
`@nube/starter-sdui-react`, `@nube/starter-ui-skills`,
`@nube/starter-ui-chat`) compose into an end-to-end flow before
the backend (`starter-flow-node-ai-builder`) lands.

Pages are AI-authored `UiComponentTree` values streamed into a
live `<Renderer>` via a scripted `BuilderAdapter`, saved to
`localStorage` (`flow-agent:pages`), viewed read-only via the same
renderer through a no-op `<SduiHost>`, and listed in a sidebar
section that updates live in-tab and across tabs. Skills is a
drop-in `<SkillsManager>` seeded with the two reference bundles
already in the repo.

## In scope

- **Lib layer** — `src/lib/pages-store.ts` (localStorage CRUD +
  `useSyncExternalStore` subscribe with a `window.storage`
  listener), `src/lib/builder-fixture.ts` (a
  `createFixtureBuilderAdapter` keyed by prompt prefix with pinned
  timings), and `src/lib/sdui-shim.tsx` (`<SduiHost>` wrapping
  `SduiProvider` with a no-op action dispatcher).
- **Route screens** — `PagesList`, `PageBuilder`, `PageView`,
  `Skills` under `src/pages/`. `PageBuilder` **composes**
  `useBuilder` + `<BuilderTranscript>` + `<AiBuilderCanvas>`
  directly so the host owns the current `tree` and can wire the
  save button (the opinionated `<AiBuilder>` wrapper hides
  `tree`).
- **Shell wiring** — five new routes (`/pages`, `/pages/new`,
  `/pages/:id`, `/pages/:id/edit`, `/skills`) in `app.tsx`; one
  collapsible **Pages** section subscribed to `pages-store` plus
  one **Skills** leaf in `Shell.tsx`.
- **Eight acceptance checks** from PAGE-BUILDER.md all pass.

## Out of scope

- **Any new Rust crates or backend changes.** The slice is
  purely frontend; the Rust backend lands later as
  `starter-flow-node-ai-builder` per `DOCS/frontend/ai-builder/SCOPE.md`.
- **Server-persisted pages.** `localStorage` is the contract for
  v0.1; a `PagesStore` trait + REST handler is a future
  iteration.
- **The theme-builder slice** (`@nube/starter-ai-builder-react-theme`).
  Reuses the same `BuilderEvent` shape but ships separately.
- **A real LLM.** Output is scripted via the fixture adapter so
  the demo is reproducible and works offline. Swapping in an SSE
  adapter is additive when the backend lands.
- **Edits to `frontend/package.json`.** Every `@nube/*` lib used
  here is already a dep.
- **Wrapping the opinionated `<AiBuilder>`.** It hides
  `useBuilder` internals; the save button needs `tree`.

## Hard rules (load-bearing)

- **R1** — `PageBuilder.tsx` composes `useBuilder` directly. Do
  **not** wrap `<AiBuilder>`; the wrapper owns `useBuilder`
  internally and the host can't reach `tree` to wire 💾. The
  `ai-builder.tsx` source itself recommends this for "full
  control".
- **R2** — Sidebar live-update is `useSyncExternalStore` over
  `pages-store` plus a `window.storage` listener. Pure
  `localStorage` reads do not trigger React re-renders, and the
  `storage` event covers cross-tab.
- **R3** — Fixture timings are pinned and deterministic. The
  buffered-patch demo uses `patch` at t=0ms and parent
  `full_render` at t=80ms, well inside the default `useBuilder`
  buffer window. The "buffered" badge text in the screenshot is
  illustrative; the actual count is whatever the hook reports.
- **R4** — `<SduiHost>` is a thin wrapper around `SduiProvider`
  from `@nube/starter-sdui-react` that installs a no-op action
  dispatcher. The same shim is reused by the builder canvas and
  `/pages/:id` so saved trees with interactive nodes don't blow
  up at runtime.
- **R5** — `frontend/package.json` is unchanged. Adding a new
  `@nube/*` dep here means the slice is straying out of scope.
- **R6** — Pages live in `localStorage` keyed `flow-agent:pages`
  as `{ id, name, tree, createdAt }`. The `tree` field is the
  exact `UiComponentTree` from the wire format, so Edit ⇄ View
  round-trip is lossless.

## Constraints

- **No Rust changes.** Only files under
  `examples/flow-agent/frontend/src/` (and possibly tests).
- **Use the libs as published.** `<Renderer>` from
  `@nube/starter-sdui-react`; `useBuilder` /
  `<BuilderTranscript>` / `<AiBuilderCanvas>` from
  `@nube/starter-ui-ai-builder`; `<SkillsManager>` +
  `createInMemorySkillsAdapter` from `@nube/starter-ui-skills`.
- **`<Renderer node={page.tree.root} />`** — `UiComponentTree.root`
  is the entry point per
  `packages/starter-sdui-react/src/types.ts`.
- **Empty state matters.** First visit to `/pages` shows "No
  pages yet — hit + New page to start", not the three-card
  screenshot. The screenshot is illustrative for a populated
  state.
- **Buffered-patch demo is deterministic.** The R1 buffer drops
  the patch silently after a window elapses (see
  `packages/starter-ui-ai-builder/src/hooks/use-builder.ts`); if
  the parent `full_render` arrives later than the window the demo
  silently breaks. The pinned timings keep it green.
- **Skills approve refresh is built in.** `<SkillsManager>`
  refetches on mutation success against `createInMemorySkillsAdapter`,
  which mutates state in place and surfaces it on the next
  fetch. Confirm in stage 1 that the manager actually issues the
  refetch; if it doesn't, that's a bug to fix in
  `@nube/starter-ui-skills` before this slice ships.

## Phasing

- **Stage 1** — pin the four open design points (sidebar
  live-update, fixture timings, `<SduiHost>` shape, no
  package.json edit). No code.
- **Stages 3–4** — lib layer, then route screens.
- **Stage 5** — routes + shell nav + acceptance sweep.

## Deliverables

- `src/lib/pages-store.ts`, `src/lib/builder-fixture.ts`,
  `src/lib/sdui-shim.tsx`.
- `src/pages/PagesList.tsx`, `src/pages/PageBuilder.tsx`,
  `src/pages/PageView.tsx`, `src/pages/Skills.tsx`.
- Updated `src/app.tsx` (five new routes) and
  `src/layout/Shell.tsx` (Pages section + Skills leaf).
- All eight acceptance checks in PAGE-BUILDER.md pass.

## Open questions (resolve in stage 1)

The source PAGE-BUILDER.md does not enumerate explicit open
questions, but landing the slice cleanly requires the runner to
pin four small design points before code starts:

1. **Sidebar live-update mechanism.** Bias:
   `useSyncExternalStore` over a `pages-store` `subscribe(fn)`
   helper (in-process listeners) plus a `window.storage` event
   listener so cross-tab changes propagate. Pure `localStorage`
   reads do not trigger React re-renders.
2. **Buffered-patch fixture timings.** Bias: `patch` at t=0ms,
   parent `full_render` at t=80ms, phase `done` < 2s. Well
   inside the default `useBuilder` buffer window so the badge
   shows up briefly and then resolves cleanly.
3. **`<SduiHost>` shape.** Bias: a thin wrapper around
   `SduiProvider` from `@nube/starter-sdui-react` that installs a
   no-op action dispatcher. Same shim is reused by the builder
   canvas and `/pages/:id`.
4. **`frontend/package.json` edits.** Bias: none. Every `@nube/*`
   lib used here is already a dep. If stage 1 surfaces a missing
   dep, document it and re-scope.
