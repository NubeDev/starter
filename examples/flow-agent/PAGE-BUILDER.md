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

Two new sidebar sections plug into the existing `flow-agent` shell.
Everything is fixtures + `localStorage`; no new Rust crates, no
backend changes.

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

Built from `<AiBuilder>` (`@nube/starter-ui-ai-builder`). Chat on
left, live `<Renderer>` canvas on right. A scripted `BuilderAdapter`
streams `BuilderEvent`s so the tree builds progressively:
`status: thinking` → `full-render` skeleton → 3–5 `patch` events
filling sections → `status: done`.

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
| `sales`     | KPI grid + pipeline table (3 streamed patches)               |
| `dashboard` | Same as `sales`                                              |
| `onboard`   | Form + checklist + tabs                                      |
| `report`    | Heading + markdown + chart + table                           |
| _anything_  | Minimal hello card (never feels broken)                      |

The R1 patch-buffer (per ai-builder SCOPE) is exercised: one fixture
deliberately fires `patch` before its parent `full-render` to show
the "2 buffered" badge resolve cleanly.

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

Same `SduiProvider` shim as the builder canvas (no-op actions) —
proves the saved tree round-trips through the wire format unchanged.

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
  package.json                                ✎ +3 workspace deps
  src/app.tsx                                 ✎ +4 routes
  src/layout/Shell.tsx                        ✎ Pages + Skills nav
  src/lib/
    pages-store.ts                            + localStorage CRUD
    builder-fixture.ts                        + scripted BuilderAdapter
    sdui-shim.tsx                             + tiny <SduiHost> wrapper
  src/pages/
    PagesList.tsx                             +
    PageBuilder.tsx                           + wraps <AiBuilder/>
    PageView.tsx                              + wraps <Renderer/>
    Skills.tsx                                + wraps <SkillsManager/>
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

1. `pnpm --filter flow-agent-frontend dev` → `/pages` loads.
2. `+ New page` → type `sales` → tree streams in, "2 buffered" badge
   briefly visible, lands at done in <2s.
3. 💾 Save → redirected to `/pages/:id` → same tree renders, no
   layout shift.
4. Sidebar updates live (new page appears under "Pages").
5. `/skills` shows two bundles; approving the quarantined one moves
   it to "Approved" without a refresh.
6. `pnpm typecheck` is clean.
7. Lighthouse on `/pages/:id` ≥ 95 performance; no console errors.
