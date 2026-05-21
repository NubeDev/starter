# flow-agent — Page Builder slice

A reference UX for **AI-authored SDUI pages**, layered onto the
existing `flow-agent` shell with **zero new Rust code**. Exists to
prove the four reusable React libs compose into an "amazing"
end-to-end flow before the backend (`starter-flow-node-ai-builder`)
lands.

> Companion to:
> - [DOCS/frontend/sdui/SCOPE.md](../../DOCS/frontend/sdui/SCOPE.md) — wire format
> - [DOCS/frontend/ai-builder/SCOPE.md](../../DOCS/frontend/ai-builder/SCOPE.md) — authoring mode
> - [DOCS/agent/SKILLS.md](../../DOCS/agent/SKILLS.md) — skill registry
> - [SCOPE.md](./SCOPE.md) — host example

---

## What it ships

One new collapsible sidebar section (**Pages**) and one new leaf
(**Skills**) plug into the existing `flow-agent` shell. Everything is
fixtures + `localStorage`; no new Rust crates, no backend changes,
no new workspace dependencies (the four `@nube/*` libs below are
already in [frontend/package.json](frontend/package.json)).

```
┌─────────────────────────────────────────────────────────────────────────┐
│  flow-agent                                            ☀  operator ▾    │
├──────────────┬──────────────────────────────────────────────────────────┤
│ ▾ Flows      │                                                          │
│   • onboard  │                                                          │
│   • report   │                                                          │
│ ▾ Agents     │                                                          │
│   • Assistant│                  (route outlet)                          │
│ ▾ Pages   ★  │                                                          │
│   • Sales KPI│                                                          │
│   • Onboard  │                                                          │
│ • Skills  ★  │                                                          │
│ • Settings   │                                                          │
└──────────────┴──────────────────────────────────────────────────────────┘
        ★ = new
```

## Routes

| URL                    | Page              | What you see                                            |
| ---------------------- | ----------------- | ------------------------------------------------------- |
| `/pages`               | **PagesList**     | Cards/table of saved pages; "+ New page" opens builder  |
| `/pages/new`           | **PageBuilder**   | Split-pane: chat transcript ← → live SDUI canvas        |
| `/pages/:id`           | **PageView**      | Read-only render of a saved page via `<Renderer>`       |
| `/pages/:id/edit`      | **PageBuilder**   | Same builder seeded with the saved tree                 |
| `/skills`              | **SkillsManager** | List/inspect/approve/revoke skill bundles               |

---

## 1. `/pages` — PagesList

```
┌───────────────────────────────────────────────────────────────────┐
│ Pages                                                3 total      │
│ AI-built dashboards rendered with SDUI.       [+ New page]        │
├───────────────────────────────────────────────────────────────────┤
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐       │
│ │ Sales KPI       │ │ Onboarding      │ │ Daily report    │       │
│ │ 4 KPIs · table  │ │ Form · checklist│ │ Charts · table  │       │
│ │ updated 2m ago  │ │ updated 1h ago  │ │ updated yest.   │       │
│ │  [View] [Edit] ⋯│ │  [View] [Edit] ⋯│ │  [View] [Edit] ⋯│       │
│ └─────────────────┘ └─────────────────┘ └─────────────────┘       │
└───────────────────────────────────────────────────────────────────┘
```

## 2. `/pages/new` — PageBuilder (the headline screen)

**Composed**, not the opinionated `<AiBuilder>`. The opinionated
wrapper owns `useBuilder` internally, so the host can't reach the
current `tree` to wire the 💾 save button. Instead `PageBuilder.tsx`
calls `useBuilder` itself and lays out `<BuilderTranscript>` +
`<AiBuilderCanvas>` side-by-side — the `ai-builder.tsx` source
recommends exactly this for "full control".

A scripted `BuilderAdapter` streams `BuilderEvent`s so the tree
builds progressively: `status: thinking` → `full_render` skeleton →
3–5 `patch` events filling sections → `status: done`.

```
┌───────────────────────────────────────────────────────────────────┐
│ Page Builder              [thinking|writing|done] 2 buffered  💾  │
├──────────────────────────┬────────────────────────────────────────┤
│  ▸ "Sales dashboard for  │   ┌──────────────────────────────────┐ │
│    Q2 with 4 KPIs and a  │   │ Sales · Q2                       │ │
│    pipeline table"       │   ├──────────────────────────────────┤ │
│                          │   │  ┌─────┐┌─────┐┌─────┐┌─────┐   │ │
│  ● Planning…             │   │  │ MRR ││ ARR ││ Win ││ NPS │   │ │
│  ● Writing layout…       │   │  │$42k ││$508k││ 31% ││ 62  │   │ │
│  ● Done                  │   │  └─────┘└─────┘└─────┘└─────┘   │ │
│                          │   │  Pipeline                        │ │
│  [● ● ●  Working…]       │   │  ┌────────────────────────────┐  │ │
│                          │   │  │ Stage    Deals   Value     │  │ │
│  ┌─────────────────────┐ │   │  │ Qual     12     $84k       │  │ │
│  │ Describe the UI…  ➤│ │   │  │ Demo      8     $112k      │  │ │
│  └─────────────────────┘ │   │  │ Close     3     $96k       │  │ │
│   [Cancel] [↻ Regen]     │   │  └────────────────────────────┘  │ │
│                          │   └──────────────────────────────────┘ │
└──────────────────────────┴────────────────────────────────────────┘
```

Top-right 💾 = "Save page" → writes `{ id, name, tree, createdAt }`
into `localStorage` under `flow-agent:pages`, then navigates to
`/pages/:id`.

**Prompts the fixture adapter recognises** (matched by prefix):

| Prefix      | Result                                                       |
| ----------- | ------------------------------------------------------------ |
| `sales`     | Q2 KPI grid + pipeline table (3 streamed patches)            |
| `dashboard` | Generic 2×2 KPI grid + line chart (distinct from `sales`)    |
| `onboard`   | Form + checklist + tabs                                      |
| `report`    | Heading + markdown + chart + table                           |
| _anything_  | Minimal hello card (never feels broken)                      |

The R1 patch-buffer (per ai-builder SCOPE) is exercised: the `sales`
fixture deliberately fires one `patch` *before* its parent
`full_render` to show the "buffered" badge resolve cleanly. Timing
is pinned so the demo is deterministic — `patch` at t=0 ms, parent
`full_render` at t=80 ms, well inside the default buffer window
(see [use-builder.ts](../../packages/starter-ui-ai-builder/src/hooks/use-builder.ts)).
The badge text in the screenshot ("2 buffered") is illustrative; the
actual count depends on the fixture and is whatever the hook reports.

## 3. `/pages/:id` — PageView

```
┌───────────────────────────────────────────────────────────────────┐
│ Sales KPI                            [Edit] [Duplicate] [Delete]  │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│   <Renderer node={page.tree.root} />  ← same renderer as builder  │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

Wrapped in a local `<SduiHost>` (`src/lib/sdui-shim.tsx`) — a thin
wrapper around `SduiProvider` from `@nube/starter-sdui-react` that
installs a no-op action dispatcher, so Buttons and other
interactive nodes in saved trees don't blow up at runtime. Same
shim is reused by the builder canvas, which proves the saved tree
round-trips through the wire format unchanged.

## 4. `/skills` — Skills

Drop-in `<SkillsManager adapter={…} />` (`@nube/starter-ui-skills`),
seeded with the two reference bundles already in the repo
(`starter.ai-builder.dashboards`, `starter.ai-builder.themes`) via
`createInMemorySkillsAdapter`.

```
┌───────────────────────────────────────────────────────────────────┐
│ Skills                                       [⟳ Reload]           │
│ [All] [Approved] [Quarantined]   🔍 search…                       │
├──────────────────────────┬────────────────────────────────────────┤
│ ● starter.ai-builder.    │  starter.ai-builder.dashboards         │
│   dashboards  [approved] │  ─────────────────────────────────     │
│ ○ starter.ai-builder.    │  Drafts, edits, and publishes…         │
│   themes      [quarantd] │  hash 9f1e8b2c…   source host          │
│                          │                                        │
│                          │  Allowed tools                         │
│                          │  • starter.mcp.call                    │
│                          │  • starter.flow.transform              │
│                          │                                        │
│                          │  Body  │  Resources (2)                │
│                          │  ─────────────────                     │
│                          │  # Dashboards skill … (verbatim md)    │
│                          │                                        │
│                          │            [Revoke approval]           │
└──────────────────────────┴────────────────────────────────────────┘
```

---

## Files

```
examples/flow-agent/frontend/
  src/app.tsx                                 ✎ +4 routes
  src/layout/Shell.tsx                        ✎ Pages + Skills nav,
                                                subscribes to pages-store
  src/lib/
    pages-store.ts                            + localStorage CRUD +
                                                useSyncExternalStore hook
                                                (subscribe + window
                                                 'storage' listener for
                                                 cross-tab sync)
    builder-fixture.ts                        + scripted BuilderAdapter
                                                (deterministic timings)
    sdui-shim.tsx                             + <SduiHost> = SduiProvider
                                                + no-op action dispatcher
  src/pages/
    PagesList.tsx                             + cards + empty state
    PageBuilder.tsx                           + composes useBuilder +
                                                <BuilderTranscript> +
                                                <AiBuilderCanvas> +
                                                💾 save button
    PageView.tsx                              + wraps <Renderer/>
    Skills.tsx                                + wraps <SkillsManager/>

Note: `frontend/package.json` is unchanged — every `@nube/*` lib
used here is already a dependency.
```

## Reusable libs exercised

| Library                          | Used for                                              |
| -------------------------------- | ----------------------------------------------------- |
| `@nube/starter-ui-ai-builder`    | `<AiBuilder>`, `useBuilder`, `BuilderAdapter`, R1 buf |
| `@nube/starter-sdui-react`       | `<Renderer>`, `SduiProvider`, `UiComponentTree` types |
| `@nube/starter-ui-skills`        | `<SkillsManager>`, `createInMemorySkillsAdapter`      |
| `@nube/starter-ui-chat`          | (transitive — transcript composer)                    |
| `@nube/starter-ui-kit`           | Cards, Tables, Buttons, Sidebar primitives            |

---

## What this is NOT

- **Not a real LLM.** Output is scripted via the fixture adapter so
  the demo is reproducible and works offline. Swap
  `createFixtureBuilderAdapter` for an SSE adapter the day
  `starter-flow-node-ai-builder` (per ai-builder SCOPE) lands; the
  rest of the UI is unchanged.
- **Not server-persisted.** Pages live in `localStorage` keyed
  `flow-agent:pages`. Clearing site data resets them. A future
  iteration can swap in a `PagesStore` trait + REST handler when a
  consumer needs multi-device sync.
- **Not the theme-builder slice.** ai-builder SCOPE describes a
  second slice (`@nube/starter-ai-builder-react-theme`) that streams
  `TokenPatch` events into the theme editor. Out of scope here;
  reuses the same `BuilderEvent` wire shape and the same patterns,
  so adding it later is additive.

## Acceptance ("works fucking amazing")

1. `pnpm --filter flow-agent-frontend dev` → `/pages` loads. First
   visit shows an empty state ("No pages yet — hit **+ New page**
   to start"); not the three-card screenshot.
2. `+ New page` → type `sales` → tree streams in, the buffered-patch
   badge is briefly visible, phase lands at `done` in <2 s.
3. 💾 Save → redirected to `/pages/:id` → same tree renders, no
   layout shift, no console errors.
4. **Round-trip:** click `Edit` on the saved page → builder loads
   with the saved tree as `initialTree` → no diff vs the rendered
   `/pages/:id` view (proves the wire format is lossless).
5. Sidebar "Pages" section updates live the moment a page is saved
   or deleted, in the same tab and across tabs (Shell subscribes to
   `pages-store` via `useSyncExternalStore` + a `window.storage`
   listener).
6. `/skills` shows two bundles; approving the quarantined one moves
   it to "Approved" without a manual reload (the manager refetches
   on mutation success).
7. `pnpm --filter flow-agent-frontend typecheck` is clean.
8. `pnpm --filter flow-agent-frontend build` succeeds; running
   Lighthouse against `vite preview` of that build on `/pages/:id`
   reports no console errors and no layout-shift warnings.
   (Performance scoring is not gated — Lighthouse against `vite dev`
   is meaningless and `vite preview` numbers vary by host.)
