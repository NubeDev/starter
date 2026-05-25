# Plan — Bring the ClickHouse Explorer into the Rubix Shell

> **Tier:** plan, not system-as-it-is. Lives in `docs/scope/` per
> [HOW-TO-CODE.md §0a](../../HOW-TO-CODE.md). Source code must not
> reference this file — once a PR below lands, its design moves into
> `docs/design/warehouse/explorer/README.md` and code links there.
>
> **Supersedes the "PR 4" rubix-overlays section of
> [`clickhouse-explorer.md`](./clickhouse-explorer.md).** PRs 0–3 of
> that plan still stand (they delivered the backend, the static-asset
> rail, and a standalone SPA). This doc replaces what was going to be
> "rubix overlays grafted onto the standalone SPA" with a real
> integration into the rubix admin shell.

## Why this exists

PR 3 of the upstream plan landed the explorer as a **separate Vite
app** under `packages/starter-ui-ch-explorer/` with its own
`main.tsx`, its own `__root.tsx` (logo, header nav, theme toggle),
its own TanStack router, its own theme provider, its own
`SqlProvider`, and its own dark-mode toggle. It is reachable at
`/warehouse/explorer/` on whichever binary statically mounts the
built `dist/`.

That is wrong for production. Operators using rubix expect the
explorer to:

1. Live inside the **rubix sidebar** at `/admin/warehouse/explorer`
   (or as a tab inside the existing `/admin/warehouse` shell), with
   the same shell, sidebar, breadcrumbs, header, user menu, and
   theme as every other rubix page.
2. Speak rubix **i18n** (`react-intl` + the `nav.item.*` /
   `admin.warehouse.*` message-key conventions) — no hard-coded
   English strings.
3. Use the rubix **design system** — `@nube/starter-ui-kit`
   primitives (`Button`, `Card`, `Dialog`, `Empty`, `Skeleton`,
   `Tabs`, …), `var(--color-leaf)` / `var(--color-muted)` /
   `var(--color-bg)` tokens — not the sql-studio shadcn fork with
   its own `Card`/`Input`/`DropdownMenu`.
4. Speak rubix **data hooks** — `@nube/rubix-client-react` typed
   verb hooks (`useClickhouseMartsList`, `useClickhouseMartDrop`,
   `useClickhouseRetentionSet`, …) for everything that mutates;
   typed read hooks (`useWarehouseStatus`, etc.) for read state.
   No bespoke `callRubixVerb` in the explorer package.
5. Follow rubix / starter **code standards** — JSDoc headers, no
   `console.log`, no `window.confirm` for destructive UX (use
   `<AlertDialog>` from the kit), `ErrorBoundary` wrapper, lazy
   route boundaries.

The standalone SPA stays useful as the **demo binary**
(`examples/ch-explorer/`) for prospects who want to see "ClickHouse
explorer over starter-server" without booting the full rubix
stack. Both surfaces share one set of UI components — that is the
whole point of the refactor.

## The two consumers

```
                 ┌────────────────────────────────────────────────┐
                 │   packages/starter-ui-ch-explorer  (library)   │
                 │                                                │
                 │   exports React components only:               │
                 │     <ExplorerOverview />                       │
                 │     <ExplorerTables />                         │
                 │     <ExplorerTableDetail name="…" />           │
                 │     <ExplorerSchema />                         │
                 │     <ExplorerQuery />                          │
                 │     <Explorer />   ← tabbed shell              │
                 │                                                │
                 │   + headless hooks:                            │
                 │     useChOverview, useChTables, useChTable,    │
                 │     useChTableData, useChQuery, useChErd,      │
                 │     useChAutocomplete                          │
                 │                                                │
                 │   + i18n: DEFAULT_EXPLORER_MESSAGES + provider │
                 │                                                │
                 │   no main.tsx, no router, no theme provider,   │
                 │   no own QueryClient, no own fetch impl.       │
                 └────────────────────────────────────────────────┘
                          ▲                          ▲
                          │                          │
            ┌─────────────┘                          └────────────┐
            │                                                     │
┌───────────────────────────┐               ┌──────────────────────────────┐
│ rubix/frontend            │               │ examples/ch-explorer-demo    │
│                           │               │ (the existing demo, slimmed) │
│ /admin/warehouse/explorer │               │                              │
│   ↳ <Explorer />          │               │ thin Vite host that wraps    │
│   wrapped in the rubix    │               │ <Explorer /> in its own      │
│   shell (sidebar, header, │               │ QueryClient + react-intl +   │
│   ErrorBoundary,          │               │ ThemeProvider so non-rubix   │
│   ThemeProvider,          │               │ deployments can still serve  │
│   QueryClient, intl,      │               │ /warehouse/explorer/.        │
│   tokens)                 │               │                              │
└───────────────────────────┘               └──────────────────────────────┘
```

One library, two mount points. The rubix shell is the **primary**
consumer; the demo binary is a fallback for non-rubix hosts.

## Layers exercised

| Layer | What changes |
|---|---|
| Library | `packages/starter-ui-ch-explorer/` flips from "Vite app" to "headless React component library" (same package name, new shape). Pattern: `@nube/starter-ui-authz`. |
| Rubix shell | New route file `rubix/frontend/src/routes/admin/warehouse.explorer.tsx` mounts `<Explorer />` inside the existing `<WarehousePanel>` layout. New sidebar entry `nav.item.warehouseExplorer`. |
| Demo binary | `examples/ch-explorer/` gains a tiny `examples/ch-explorer/ui/` host crate (or stays as-is if we can serve `rubix/frontend/dist/` from it — TBD in PR 1). |
| HTTP | No backend changes. Same `/api/warehouse/ch/*` reads, same `rubix.clickhouse.*` verbs through `POST /api/v1/tools/{tool_id}`. |
| Data hooks | Migrate explorer fetchers off zod-fetch onto `@nube/starter-client-react` / `@nube/rubix-client-react`. |
| i18n | All visible strings move to `DEFAULT_EXPLORER_MESSAGES` (mirror `DEFAULT_AUTHZ_MESSAGES`); rubix-frontend overrides via the provider. |
| Theming | Drop the `SqlProvider` + `ThemeProvider` from the library; rubix shell already owns theme. Demo host re-adds them. |

## What is deliberately out

| Cut | Why |
|---|---|
| Multiple package names | Stay with `@nube/starter-ui-ch-explorer` — same name, new exports surface. Avoids a deprecation cycle. |
| New backend routes | Backend was finished in PRs 0–2 of the upstream plan; this is pure frontend work. |
| New rubix verbs | Mart create/drop, retention set, rule write all already exist as `rubix.clickhouse.*` tools. |
| iframe embedding | Hard rejection. The rubix shell mounts `<Explorer />` as React. |
| Monaco-as-default | Keep Monaco for the Query view, but lazy-load it behind a route boundary so the rubix admin bundle doesn't grow ~2.6 MB on cold load. |
| Own router inside the library | The library exports components. The host (rubix-frontend or the demo) owns routing. |
| Own QueryClient | The host owns it. Library hooks call `useQuery` against the ambient client. |

## What stays from the standalone SPA

- The **components** under `packages/starter-ui-ch-explorer/src/components/`
  (Monaco wrapper, ERD with `@xyflow/react` + `dagre`,
  `react-data-grid` table, `sql-formatter` integration). These are
  the fork's actual value.
- The **API shape knowledge** in `src/api.ts` — zod schemas for
  `/api/warehouse/ch/*` responses move to `src/dto/` and stay
  exported (typed read hooks consume them).
- License headers (`// Forked from sql-studio (MIT) — …`) on every
  ported file. `NOTICES.md` at the workspace root stays as-is.

## What gets ripped out from the standalone SPA

- `src/main.tsx` — moves to the demo host crate.
- `src/routeTree.gen.ts`, the four `src/routes/*.tsx` files — the
  per-view content moves into `src/views/{overview,tables,schema,query}.tsx`,
  exported as components, no `createFileRoute`.
- `src/routes/__root.tsx` — its content (logo, header nav, theme
  toggle, mobile nav, dropdown menu) **deleted**. The rubix shell
  provides all of that.
- `src/provider/sql.provider.tsx` + `src/provider/theme.provider.tsx` —
  move into the demo host. The library does not declare providers.
- Per-package `tailwind` config and `tw-animate-css` dep — rubix
  owns the tailwind preset. The demo host gets its own preset
  copy (smallest viable).
- `vite.config.ts` proxy for `/api/warehouse/ch` — moves to the demo
  host. Rubix's `vite.config.ts` already proxies `/api/*` to the
  rubix-agent backend.

## Five PRs

Each PR is independently shippable. Each lights up another seam.

### PR 1 — extract the headless library

Goal: `packages/starter-ui-ch-explorer/` exports a React component
+ hook surface only; nothing in it boots, mounts, or routes.

- Rewrite `package.json`:
  - `"main"`, `"types"`, `"exports"` mirror
    `packages/starter-ui-authz/package.json` (subpath exports for
    `.`, `./hooks`, `./views`, `./i18n`).
  - Delete `dev`/`preview`/`build` Vite scripts; keep
    `"build": "tsc -p tsconfig.json --noEmit"` and `"typecheck"`.
  - Move `react`, `react-dom`, `@tanstack/react-query`,
    `@tanstack/react-router`, `@nube/starter-client-react` to
    `peerDependencies` (host supplies them).
- Delete `index.html`, `vite.config.ts`, `main.tsx`,
  `routeTree.gen.ts`, `src/routes/__root.tsx`.
- Move `src/routes/{index,tables,schema,query}.tsx` → `src/views/`
  as plain component exports.
- Replace `src/api.ts` zod-fetch with `useQuery` hooks under
  `src/hooks/` keyed off the host's QueryClient. Each hook returns
  the same zod-validated shape it does today.
- Add `src/i18n/{messages.ts,context.tsx,index.ts}` (mirror
  `starter-ui-authz/src/i18n/`). Move every visible string out of
  the views into `DEFAULT_EXPLORER_MESSAGES`.
- Add a tabbed `<Explorer />` shell in `src/views/explorer.tsx`
  built from `Tabs`/`TabsList`/`TabsTrigger`/`TabsContent` of
  `@nube/starter-ui-kit` — same shape as
  `rubix/frontend/src/components/admin/warehouse/warehouse-admin.tsx`.
- `src/index.ts` re-exports `./views`, `./hooks`, `./i18n`.

**Acceptance:** `pnpm -F @nube/starter-ui-ch-explorer typecheck`
passes. No `main.tsx`. No `RouterProvider`. No `QueryClient`
construction.

### PR 2 — port the destructive surfaces to rubix-client-react

Goal: every mutation in the library is a typed
`@nube/rubix-client-react` hook call. No raw `fetch`, no
`callRubixVerb`, no `window.confirm`.

- Replace the in-package `callRubixVerb` + `callMartList` /
  `callMartDrop` with `useClickhouseMartsList()` /
  `useClickhouseMartDrop()` from `@nube/rubix-client-react`
  (already used by `rubix/frontend/src/components/admin/warehouse/marts-panel.tsx`).
- Add hooks where they don't yet exist in `@nube/rubix-client-react`
  (sandbox list, cleaner list, retention set, rule write) — generate
  via the existing rubix-client codegen pipeline.
- Replace `window.confirm` with `<AlertDialog>` from the kit; copy
  the existing "DROP mart? This cannot be undone." prompt from
  `marts-panel.tsx` so phrasing stays consistent.
- Delete `RUBIX_VERB_NOT_AVAILABLE` sentinel + the
  `VerbOutcome<T>` discriminated union. Hooks already handle
  "verb not mounted" by surfacing the upstream error envelope.
- Delete `RUBIX_BASE` / `RUBIX_VERB_BASE` constants — the rubix
  client owns base-URL resolution.

**Acceptance:** `grep -r 'window.confirm\|callRubixVerb\|fetch(' src/`
returns nothing under `packages/starter-ui-ch-explorer/`.

### PR 3 — mount in the rubix shell

Goal: a clickable **Warehouse explorer** entry in the rubix
sidebar that renders the full explorer inside the same chrome as
`/admin/warehouse`.

- Add `@nube/starter-ui-ch-explorer: "workspace:*"` to
  `rubix/frontend/package.json` dependencies.
- Add route `rubix/frontend/src/routes/admin/warehouse.explorer.tsx`:
  ```tsx
  import { createFileRoute } from '@tanstack/react-router'
  import { Explorer } from '@nube/starter-ui-ch-explorer'
  import { ErrorBoundary } from '@/components/error-boundary'
  // … reuse the eyebrow/title pattern from warehouse.tsx
  export const Route = createFileRoute('/admin/warehouse/explorer')({
    component: () => (
      <ErrorBoundary>
        <section className="…">
          <header>…</header>
          <Explorer />
        </section>
      </ErrorBoundary>
    ),
  })
  ```
- Extend `rubix/frontend/src/lib/nav.ts` admin group with
  `{ labelKey: 'nav.item.warehouseExplorer',
     href: '/admin/warehouse/explorer', icon: Database }`
  (or fold it as a tab inside `<WarehouseAdmin>` — decide in the
  PR based on whether the operator should jump between
  rules/marts/retention/insights and the explorer or treat them
  as separate destinations).
- Add the i18n entries (`nav.item.warehouseExplorer`,
  `admin.warehouse.explorer.title`, `admin.warehouse.explorer.subtitle`)
  to every locale catalogue under `rubix/frontend/src/i18n/`.
- Pipe the rubix i18n catalogue into the explorer's
  `<ExplorerI18nProvider value={…}>` so strings like
  "Tables", "Run query", "Drop mart" pick up rubix's translations.

**Acceptance:** `cd rubix && make restart` then open
`http://localhost:5173/admin/warehouse/explorer`. The rubix
sidebar is visible, the page header reads "Warehouse explorer",
and the four explorer views (Overview / Tables / Query / Schema)
all load real data from rubix-agent.

### PR 4 — slim the demo binary

Goal: `examples/ch-explorer/` keeps working as a "no-rubix host"
demo but no longer ships a parallel Vite bundle.

- New crate `examples/ch-explorer/ui/` (or inline under the
  existing crate's `frontend/`): a 30-line `main.tsx` that wraps
  `<Explorer />` in `QueryClientProvider`, `IntlProvider`, the
  starter `ThemeProvider`, and a single TanStack route.
- Move the proxy + `BASE` config from the old
  `packages/starter-ui-ch-explorer/vite.config.ts` into this new
  host.
- Update `examples/ch-explorer/src/main.rs::serve` `DEFAULT_DIST`
  to point at the new host's `dist/`.
- Update `examples/ch-explorer/README.md` accordingly.

**Acceptance:** `cargo run -p ch-explorer-example -- serve`
followed by `curl /warehouse/explorer/` still returns 200; the
SPA still loads and renders against a local CH.

### PR 5 — promote design doc, retire the supersession note

Goal: clean documentation tail.

- Move the relevant "Landed" sections out of this scope doc and
  into `rubix/docs/design/warehouse/explorer/README.md` (which
  already exists from upstream PR 4 prep).
- Update `mod.rs` waiver in `crates/starter-warehouse/src/explorer/`
  if it still points anywhere stale.
- Delete the "Supersedes …" header above when the upstream scope
  doc's PR-4 section is rewritten to point here.

## Code-standard checklist (apply to every PR)

- [ ] Every file keeps its `// Forked from sql-studio (MIT) — …`
      header where applicable.
- [ ] No `console.log`. Use the kit's `<Empty>` / `<Skeleton>` for
      empty + loading states.
- [ ] No `window.confirm`. Use `<AlertDialog>`.
- [ ] No raw `fetch` from the library. Use hooks from
      `@nube/starter-client-react` / `@nube/rubix-client-react`.
- [ ] No hard-coded operator-facing English. Route everything
      through `useExplorerMessages()` (PR 1) or `useIntl()` in the
      rubix host.
- [ ] No new dep on a parser, no relaxation of the
      `forbid_raw_insert` / `fetch_json` write-verb refusal.
- [ ] Source code links to `docs/design/…`, never to
      `docs/scope/…` (HOW-TO-CODE §0a).
- [ ] `pnpm -w typecheck` and `cargo test -p starter-warehouse`
      stay green.

## Status (2026-05-25)

- ⏸ **PR 1** — extract headless library. Not started.
- ⏸ **PR 2** — port destructive surfaces to rubix-client-react.
  Not started.
- ⏸ **PR 3** — mount in rubix shell. Not started.
- ⏸ **PR 4** — slim the demo binary. Not started.
- ⏸ **PR 5** — promote design doc, retire supersession note.
  Not started.

## Risks and open questions

- **`@nube/rubix-client-react` hook coverage.** Mart list/drop and
  retention set already exist; sandbox-list and cleaner-list may
  not. PR 2 may need to regenerate the client. If a verb is
  missing, file the gap and stub the corresponding panel as
  "coming soon" — do not invent a non-typed call path.
- **Tab vs. sidebar entry.** Folding the explorer in as a fifth
  tab inside `<WarehouseAdmin>` keeps everything under one route;
  a separate sidebar entry keeps the explorer prominent. PR 3
  chooses one; the other lives in a follow-up.
- **Monaco bundle size.** Lazy-load the Query view so the rubix
  admin route doesn't grow by ~2.6 MB. TanStack Router supports
  per-route code splitting; use it.
- **Theming.** Rubix uses CSS variables (`var(--color-leaf)`,
  `var(--color-muted)`, `var(--color-bg)`); sql-studio uses
  shadcn's HSL token convention. PR 1 needs to translate every
  copied component to the rubix tokens, or import the rubix kit
  primitives outright (preferred).
