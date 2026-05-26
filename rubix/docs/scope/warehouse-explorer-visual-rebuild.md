# Scope — Warehouse Explorer visual rebuild (sql-studio parity)

> **Status:** scoped 2026-05-26. Supersedes the visual-parity goals of
> [`clickhouse-explorer-in-rubix-shell.md`](clickhouse-explorer-in-rubix-shell.md)
> PR 1 only. Backend, hooks, and routing decisions from that doc still
> stand.

## Why

The current explorer at `/admin/warehouse` → **Explorer** tab does not
visually resemble [frectonz/sql-studio](https://github.com/frectonz/sql-studio).
The PR-1 refactor swapped sql-studio's shadcn fork for
`@nube/starter-ui-kit` primitives 1:1, which compiled but lost:

- The framed `react-data-grid` query result panel (replaced by a
  borderless text dump — see screenshot 2026-05-26).
- Auto-execute `<Toggle>` + `<Play>` Execute button row.
- Error / empty-state `<Card>`s with `ShieldX` / `Database` icons.
- Sortable `@tanstack/react-table` rendering in the Tables view.
- Pill-shaped icon tabs (`Home`/`Table`/`Code`/`Network`) — degraded
  to plain uppercase text links with no active indicator.
- Card framing, consistent radii, and the dense JetBrains Mono layout
  that gives sql-studio its identity.

Root cause: PR-1's acceptance gate was structural (`typecheck passes,
no main.tsx`). There was no visual acceptance. The replacement kit
primitives have different paddings, radii, and no `DataGrid` analogue.

## Decision

1. **New package** `packages/starter-ui-warehouse-explorer/` — fresh
   directory, fresh code. Do **not** rewrite in place.
2. **Own sidebar entry** at `/admin/warehouse-explorer` (sibling to
   `/admin/warehouse`, not nested as a tab). Full page width.
3. **Pixel-equivalent to upstream apart from colors.** Keep the
   sql-studio shadcn fork inside this package. Re-skin via CSS
   variables only.
4. The old `packages/starter-ui-ch-explorer/` is **deleted** in the
   final PR of this work, after the demo binary at
   [`examples/ch-explorer/ui/`](../../../examples/ch-explorer/ui/) is
   repointed.

## Non-goals

- No replacing shadcn `Card`/`Button`/`Tabs`/`DataTable`/`DataGrid`
  with kit equivalents — they are the visual identity.
- No backend changes. Same `/api/warehouse/ch/*` reads, same
  `rubix.warehouse.*` verbs.
- No new monaco/erd/data-grid dependencies — they already exist in
  the old package and move over verbatim.
- No iframe, no standalone Vite dev server inside rubix-frontend.
- No i18n initially — keep upstream English strings. i18n is a
  follow-up after visual parity is signed off.

## Source-of-truth files to copy verbatim

From `/tmp/sql-studio/ui/src/` (upstream, MIT, commit pinned in
`NOTICES.md`):

| Upstream path | New location | Verbatim? |
|---|---|---|
| `components/ui/*.tsx` (15 files) | `components/ui/` | yes |
| `components/editor.tsx` + `editor.config.ts` | `components/` | yes |
| `components/erd/*` | `components/erd/` | yes |
| `components/info-card.tsx` | `components/` | yes |
| `lib/*` | `lib/` | yes |
| `routes/__root.tsx` | `views/explorer-shell.tsx` | strip TanStack `createRootRoute`; export as component; keep the nav, logo slot, theme toggle slot |
| `routes/index.tsx` | `views/overview.tsx` | strip `createFileRoute`; export as component |
| `routes/tables.tsx` | `views/tables.tsx` | same |
| `routes/query.tsx` | `views/query.tsx` | same |
| `routes/schema.tsx` | `views/schema.tsx` | same |
| `provider/sql.provider.tsx` | `providers/` | yes (host wraps `<Explorer/>` in it) |
| `provider/theme.provider.tsx` | **drop** | rubix shell owns theme; map `dark` class through |
| `index.css` | `theme.css` | keep `@theme inline`; rewrite the `--background`/`--foreground`/`--primary`/… token assignments to map to rubix tokens (see token map below) |
| `api.ts` | **drop**; replaced | see "Data layer" below |
| `main.tsx`, `routeTree.gen.ts` | **drop** | host owns routing |

License header on every copied file:

```ts
// Forked from sql-studio (https://github.com/frectonz/sql-studio) — MIT.
// Upstream commit: <pin in NOTICES.md>. Local edits: re-skinned to rubix tokens; data layer swapped to @nube/rubix-client-react.
```

## Token map (the only visual delta)

In `theme.css`, replace upstream's HSL defaults with rubix tokens.
Rubix tokens live in `rubix/frontend/src/styles/tokens.css` —
check exact values there before writing; the table below is the
intended mapping, not the literal CSS.

| shadcn token | Light mode source | Dark mode source |
|---|---|---|
| `--background` | `var(--color-bg)` | `var(--color-bg)` |
| `--foreground` | `var(--color-fg)` | `var(--color-fg)` |
| `--card` | `var(--color-surface)` | `var(--color-surface)` |
| `--card-foreground` | `var(--color-fg)` | `var(--color-fg)` |
| `--primary` | `var(--color-leaf)` | `var(--color-leaf)` |
| `--primary-foreground` | `var(--color-leaf-fg)` | `var(--color-leaf-fg)` |
| `--muted` | `var(--color-muted)` | `var(--color-muted)` |
| `--muted-foreground` | `var(--color-muted-fg)` | `var(--color-muted-fg)` |
| `--border` | `var(--color-border)` | `var(--color-border)` |
| `--input` | `var(--color-border)` | `var(--color-border)` |
| `--ring` | `var(--color-leaf)` | `var(--color-leaf)` |
| `--destructive` | rubix red (look up) | same |
| `--radius` | `0.5rem` (match rubix card radius) | same |

Dark mode trigger: rubix toggles `.dark` on `<html>`. Upstream's
`@custom-variant dark (&:is(.dark *))` already works — no change.

JetBrains Mono: keep the upstream `--font-mono` declaration. Rubix
already loads JetBrains Mono for code surfaces.

## Data layer

Swap upstream's `api.ts` (zod-fetch against `/api/warehouse/ch/*`) for
typed hooks from `@nube/rubix-client-react`. Keep response shapes
identical so views don't change.

| Upstream call | Replacement |
|---|---|
| `fetchOverview()` | `useWarehouseStatus()` |
| `fetchTables()` | `useClickhouseTables()` |
| `fetchTable(name)` | `useClickhouseTable(name)` |
| `fetchTableData(name, page)` | `useClickhouseTableData(name, page)` |
| `fetchQuery(sql)` | `useClickhouseQuery(sql)` |
| `fetchErd()` | `useClickhouseErd()` (file gap if missing — see Risks) |
| `fetchAutocomplete()` | `useClickhouseAutocomplete()` (same) |

Each hook lives in `hooks/use-*.ts`; views import the hook, not the
fetcher.

## Mount in rubix shell

1. Add `@nube/starter-ui-warehouse-explorer: "workspace:*"` to
   [`rubix/frontend/package.json`](../../frontend/package.json).
2. New route file
   `rubix/frontend/src/routes/admin/warehouse-explorer.tsx`:

   ```tsx
   import { createFileRoute } from '@tanstack/react-router'
   import { Explorer, SqlProvider } from '@nube/starter-ui-warehouse-explorer'
   import '@nube/starter-ui-warehouse-explorer/theme.css'
   import { ErrorBoundary } from '@/components/error-boundary'

   export const Route = createFileRoute('/admin/warehouse-explorer')({
     component: () => (
       <ErrorBoundary>
         <SqlProvider>
           <Explorer />
         </SqlProvider>
       </ErrorBoundary>
     ),
   })
   ```

   No outer page header — `<Explorer/>` carries its own nav bar
   (ported from upstream `__root.tsx`).

3. Sidebar entry in
   [`rubix/frontend/src/components/layout/data/sidebar-data.ts`](../../frontend/src/components/layout/data/sidebar-data.ts):

   ```ts
   { title: 'Explorer', url: '/admin/warehouse-explorer', icon: Database }
   ```

   Place it under the existing **Platform → Warehouse** group, below
   the current "Warehouse" entry.

4. Remove the "Explorer" tab from
   [`rubix/frontend/src/components/admin/warehouse/warehouse-admin.tsx`](../../frontend/src/components/admin/warehouse/warehouse-admin.tsx)
   — the explorer is no longer a tab inside the warehouse admin
   page.

## Demo binary

[`examples/ch-explorer/ui/`](../../../examples/ch-explorer/ui/)
repoints from `@nube/starter-ui-ch-explorer` to
`@nube/starter-ui-warehouse-explorer`. Wraps `<Explorer/>` in its
own `SqlProvider` + `QueryClientProvider` + theme provider (the
upstream `theme.provider.tsx` lives here, not in the library).

## Delete the old package

Final PR: delete
[`packages/starter-ui-ch-explorer/`](../../../packages/starter-ui-ch-explorer/)
and remove all `workspace:*` references from `rubix/frontend` and
`examples/ch-explorer/ui`.

## Acceptance (visual)

Side-by-side screenshots at 1440×900, rubix dark theme:

- [ ] Top tab bar shows four pill tabs with icons (`Home Overview`,
      `Table Tables`, `Code Query`, `Network Schema`); active tab
      has the leaf-coloured background.
- [ ] Query view: editor on top, `[Auto-execute toggle] [Execute]`
      row below editor, result table is a framed `<Card>` with
      `react-data-grid` rendering, resizable columns, alternating row
      shading.
- [ ] Empty query state: centered `Database` icon + "Query Executed —
      Returned no data" inside a `<Card>`.
- [ ] Error state: centered red `ShieldX` icon + "Error — Query
      didn't execute successfully." inside a `<Card>`.
- [ ] Tables view: sortable column headers, search box, pagination
      controls — `@tanstack/react-table` driven, identical layout
      to upstream apart from colours.
- [ ] All copy stays in JetBrains Mono where upstream uses it
      (column headers, query editor, code blocks).
- [ ] No layout drift between rubix `light` and `dark` modes — token
      map handles both.

## Acceptance (structural)

- [ ] `pnpm -F @nube/starter-ui-warehouse-explorer typecheck` passes.
- [ ] No `fetch(` or `callRubixVerb` in `src/`.
- [ ] No `console.log`.
- [ ] License headers on every file ported from upstream.
- [ ] `pnpm -w typecheck` green; `cargo test -p starter-warehouse`
      green.

## Risks and open questions

- **Missing `@nube/rubix-client-react` hooks.** `useClickhouseErd`
  and `useClickhouseAutocomplete` may not exist. If absent: stub the
  Schema view with an `Empty` state and a TODO; do not invent a
  fetcher.
- **Tailwind v4 vs rubix's tailwind setup.** Upstream uses
  `@tailwindcss/vite` v4 with `@theme inline`. If rubix is still on
  v3, the package needs to publish its CSS pre-built so rubix
  consumes a compiled `theme.css` rather than running v4 inside
  rubix's v3 pipeline. Check
  [`rubix/frontend/package.json`](../../frontend/package.json)
  before starting.
- **`@xyflow/react` + `dagre` bundle size.** Lazy-load the Schema
  view via `React.lazy` so the warehouse-explorer route doesn't
  cold-load ~600 KB.
- **Monaco bundle size.** Same — lazy-load the Query view.

## PR breakdown

1. **PR 1 — scaffold new package** (this scope). Copy upstream
   files verbatim, drop `main.tsx`/`routeTree.gen.ts`, add license
   headers, swap `api.ts` for `hooks/use-*.ts` stubs that return
   the same shapes, write the token-map `theme.css`. Typecheck
   passes; views render against stub data.
2. **PR 2 — wire real hooks.** Replace stub returns with
   `@nube/rubix-client-react` calls. File gaps for any missing
   verbs.
3. **PR 3 — mount in rubix.** Route file + sidebar entry + remove
   the Explorer tab from warehouse-admin. Visual acceptance gate
   here.
4. **PR 4 — repoint demo binary** to the new package; delete the
   old `packages/starter-ui-ch-explorer/`.
5. **PR 5 — i18n** (optional, follow-up). Wrap upstream strings in
   `react-intl` against rubix's existing `nav.item.*` /
   `admin.warehouse.*` catalogues.
