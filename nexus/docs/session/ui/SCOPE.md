# Nexus Frontend — Scope (`nexus/ui/`)

> **Before you write code:** read [../README.md](../README.md) — the session coding rules
> (one responsibility per file; comments say *why*, never the stage/fix/phase/rule that
> produced them; planning docs stay out of source).
>
> **Architecture source of truth:** [`../../scope/NEXUS.md`](../../scope/NEXUS.md) §7 + the existing
> mockup overview [`../../../../nexus-ui/OVERVIEW.md`](../../../../nexus-ui/OVERVIEW.md). This doc is
> the **build scope** for the new UI in `nexus/ui/` — what we ship, the file layout, the rules,
> the port plan, the phases, and the **subagent work-units**. Backend is the sibling
> [`../backend/SCOPE.md`](../backend/SCOPE.md).
>
> **Layout law:** [`../../../../rubix/FILE-LAYOUT.md`](../../../../rubix/FILE-LAYOUT.md) applies to
> `.ts`/`.tsx` too — one component/verb per file, folder-of-verbs over file-of-nouns, ≤400 lines
> hard / ~100 typical, names are concepts (never `utils.ts`/`helpers.ts`/`index.ts`-with-bodies).

## One-line summary

`nexus/ui` is the **React 19 + shadcn/ui dashboard builder** for Nexus: a drag-and-drop canvas of
**ECharts** panels backed by **TanStack Query** over the `nexus-api` REST surface, with **live
panels** over SSE, **auth/authz context**, and a **federation host** so the existing rubix
extensions (`com.nubeio.ce`) mount unchanged. It is **ported from the `nexus-ui` mockup** into
`nexus/ui/` and re-laid-out per FILE-LAYOUT.

## The mental model

**Shell = CRUD; custom code = the dashboard engine.** The shell (sidebar, dashboard records,
forms) rides the **starter client** (`starter-client-ts/-react`) — cheap, swappable, and the
*only* source of data (no fakes, F0). The engine (grid canvas + panel/widget library) is the
multi-week core. The two are split so the engine stays snappy and
the shell stays boring.

Three things are **shared singletons** the federation host owns and every extension reuses:
**React/react-dom**, **`@tanstack/react-query`** (one `QueryClient`/cache), **`zustand`** (one
client-state store). This is not preference — it's the federation contract (OVERVIEW §7). It
forces: **no Refine** (it brings its own query runtime extensions can't share), and **matching
majors** across host + remotes (hard refusal on mismatch).

The **data model** (`data/types.ts`) is the stable contract that survives every layer swap — but
it **must be extended**: today a panel is keyed by a fake `metric` string; a real panel
references a **datasource + query + field mapping**. Keep it schema-described and
side-effect-free so the OpenUI "Ask Nexus" path (OVERVIEW §8) stays open.

## What nexus/ui is, exactly

1. **Dashboard CRUD** — sidebar create/edit/star/delete; records over `nexus-api`.
2. **Drag-and-drop canvas** — `react-grid-layout` (spike dnd-kit; it's in maintenance), edit-mode
   move/resize/add/duplicate/remove, layout auto-save.
3. **Panel/widget library** — line, area, gauge, stat/KPI, status list, device table — on
   **ECharts** (the mock's Recharts is swapped).
4. **Query editor + Explore** — author a panel's datasource + SQL; one-shot `POST /query`.
5. **Live panels** — subscribe to `GET /api/v1/streams/:id` (SSE), token/cookie auth — **not**
   Bearer (native `EventSource` can't set headers).
6. **Auth/authz context** — `usePrincipal()` / `useCan()` over `GET /api/v1/me`; plain context,
   not framework providers.
7. **Federation host** — `@nube/starter-ext-ui` provider + `<ExtensionSlot>`s; mounts
   `com.nubeio.ce` unchanged; CSS scoped under `data-ext-id`.
8. **(Deferred) OpenUI "Ask Nexus"** — generative dashboards into the same `Dashboard` records.

## Hard rules (load-bearing) — F0…F11

### F0 — NO MOCK DATA. NONE. ANYWHERE IN APP CODE.
**Absolute** (README §6). Every value on screen comes from `nexus-api` through the real client.
**Do NOT port** the mock's `data/fake.ts`, `data/seed.ts`, or its `localStorage` store. No
hardcoded series/dashboards, no MSW/network mock faking responses in the running app. Endpoint
not ready? Render **loading / empty / error** — never invented rows. A fabricated value in `src/`
is a build-blocking bug. (Typed props in `*.test.tsx` are test inputs, not mock data — fine.)

### F1 — One component/verb per file. 400 lines. Always.
Per FILE-LAYOUT. One API binding per file; split big screens (`DashboardPage` → page shell +
toolbar + canvas host). Never `utils.ts` / `helpers.ts` / `index.ts` exporting bodies
(re-export only).

### F2 — Data layer = the starter client, codegen'd. No Refine, no hand-rolled fetch.
The data layer is **`@nube/starter-client-ts`** (typed HTTP client, **codegen'd from
`nexus-spi`'s OpenAPI**, zod-validated, zero React) + **`@nube/starter-client-react`** (its
TanStack Query provider + `StarterClientProvider` + shared auth/SSE hooks). Nexus-specific
endpoints are typed bindings over the client, one verb per file under `src/api/<noun>/<verb>.ts`
— **not** raw `fetch`. **Drop `@refinedev/*` entirely.** OpenAPI is the single source of truth;
hand-edited wire types are forbidden (CI fails on drift). This is what makes F0 enforceable — the
only way to get data is the real client.

### F3 — Shared singletons come from the starter packages; matching majors, hard refusal.
The federation singletons — **React 19**, **`@tanstack/react-query`** (via
`@nube/starter-client-react`), **`zustand`** + **i18n/prefs** (via `@nube/starter-ui-core`) —
are host-owned; every remote declares the same set with a version; a major mismatch is a **hard
refusal**, not a silent second copy. The host is **`@nube/starter-ext-ui`**, wrapping
`StarterClientProvider` + the `starter-ui-core` `AuthProvider`. Build on these from day one
rather than re-deriving them.

### F4 — Routing stays host-only.
React Router lives in the host; extensions are **slot contributions keyed by name**, not routes.
**TanStack *Router* is not used** — it's TanStack **Query** that's shared, not routing.

### F5 — SSE auth is not Bearer.
Native `EventSource` can't send an `Authorization` header. Live panels authenticate via a
**signed stream token in the URL** (minted by a REST call) or an `HttpOnly` cookie, or use a
fetch-based SSE reader. Mirrors backend R8 — keep the two in lockstep.

### F6 — Panels are ECharts, schema-described, side-effect-free.
A widget renders purely from its `Widget`/`WidgetConfig` + fetched data — no fetching inside a
widget, no side effects. This is what keeps the OpenUI generative path (and live re-render) free.

### F7 — `data/types.ts` is the contract; extend it, keep it stack-agnostic.
Add the real panel binding — `datasourceId`, `query` (SQL), `fields`/series mapping, and
`live?: { streamId }` — without coupling to the provider or chart lib. The mock's `metric`-keyed
config is replaced by a datasource+query reference. Zero React imports in this file.

### F8 — CSS isolation for extensions.
Render each extension's subtree under a `data-ext-id="…"` wrapper; extension CSS is injected-by-JS
and scoped to that attribute. Host Tailwind must not bleed into remotes, nor vice versa.

### F9 — Extension loading is code execution — gate it.
Loading a remote `remoteEntry.js` runs trusted code in the user's session. Before any
**out-of-repo** remote loads: manifest **allowlist**, **checksum/signature** pin, **CSP**, a
version policy, and an explicit **capability boundary on `StarterClient`**. v1 only loads the
in-repo `com.nubeio.ce`, so this can trail the host runtime — but it gates third-party remotes.

### F10 — Test-driven: the test comes first.
**Red → green → refactor**, per [../README.md](../README.md) §5. The failing test precedes the
component/hook. Pure components are tested with **typed `Widget`/DTO props** (the contract — not
mock telemetry, F0); integration runs against a **real `nexus-api`** (dev instance /
testcontainers / Playwright), never a faked network. Co-locate (`Gauge.tsx` → `Gauge.test.tsx`);
`npm run typecheck` is part of the gate.

### F11 — Reuse the starter UI platform; don't re-derive it.
Primitives = **`@nube/starter-ui-kit`** (shadcn + Tailwind v4 tokens), tokens =
**`@nube/starter-theme-tokens`**, auth/state/i18n brain = **`@nube/starter-ui-core`**, data =
**`@nube/starter-client-ts/-react`**, federation host = **`@nube/starter-ext-ui`**. Reach for an
existing `starter-ui-*` package before hand-building (query editor, authz admin, dashboard tiles
— see §What we use from starter). The `nexus-ui` mock is the **UX/visual reference**, not the
codebase to copy.

## What we use from starter

Nexus lives inside the starter monorepo, so these are in-repo workspace packages under
`packages/` (+ `starter-extensions/packages/`) — path/workspace deps, consumed not copied.

**Foundation (v1, use from day one):**

| Package | Gives us | Replaces in the mock |
|---|---|---|
| `@nube/starter-client-ts` | Codegen'd typed HTTP client (from `nexus-spi` OpenAPI), zod, zero React | the localStorage `dataProvider` |
| `@nube/starter-client-react` | TanStack Query provider + `StarterClientProvider` + auth/**SSE** hooks | Refine's query runtime |
| `@nube/starter-ui-kit` | shadcn/ui primitives + Tailwind v4 tokens + theme switch | the hand-copied `components/ui/*` |
| `@nube/starter-ui-core` | `AuthProvider`/`useAuth`, query-key namespacing, **zustand**, react-intl | component state + ad-hoc context |
| `@nube/starter-theme-tokens` | design tokens (pure data) | bespoke `index.css` token block (keep Nexus accents on top) |
| `@nube/starter-ext-ui` | federation **host** runtime + `<ExtensionSlot>` | (new — F3) |
| `@nube/starter-ext-sdk-ts` | remote SDK (what `com.nubeio.ce` already builds against) | (reference only) |

**Reuse candidates (prefer over hand-building — F11):**

| Package | Use for | Note |
|---|---|---|
| `@nube/starter-ui-warehouse-explorer` | the **query editor / Explore** (Monaco SQL editor + results grid, built on `starter-client-react`) | strong reuse; re-skin to Nexus tokens |
| `@nube/starter-ui-authz` | **teams / members / permissions** admin (tenants, grants, audit) | covers the NEXUS "assign team to a page" admin |
| `@nube/starter-ui-dashboard` | stat/KPI tiles, status/activity feeds (pure presentation) | **decision:** these use `motion`/own charts; keep **ECharts** for dense line/area/gauge, reuse tiles where they fit |

**Later / not v1:** `@nube/starter-ui-sdui-react` (only if Nexus adopts server-driven UI),
`@nube/starter-ui-ai-builder` (the OpenUI "Ask Nexus" path, P5).

> **This refines [../../scope/NEXUS.md](../../scope/NEXUS.md) §7's "plain fetch + context".** The
> starter platform already ships a better-integrated data layer (codegen'd client + TanStack
> hooks + auth + SSE) and the federation singletons (zustand/i18n via `starter-ui-core`). Use
> them; don't re-derive. **One open decision:** how much of `starter-ui-dashboard` to adopt vs
> build ECharts panels — resolve at P2.

## Source layout (`nexus/ui/src/`)

Ported from `nexus-ui/src/` and re-laid-out by feature + verb-per-file:

```
nexus/ui/
  index.html                          <- keep the process shim (react-grid-layout needs it)
  vite.config.ts                      <- federation HOST config (importmap singletons)
  package.json                        <- React 19, @tanstack/react-query, zustand, echarts; NO @refinedev/*
  src/
    main.tsx                          <- root render only
    app/
      providers.tsx                   <- StarterClientProvider (TanStack) + ExtensionHostProvider + ui-core AuthProvider
      router.tsx                      <- routes (host-only, F4)
    api/                              <- F2: typed bindings over @nube/starter-client-ts, one verb per file
      client.ts                       <- configure @nube/starter-client-ts (base URL, auth); NO raw fetch
      datasources/ {list,get,create,update,delete,test,query}.ts
      dashboards/  {list,get,create,update,delete,star}.ts
      panels/      {list,create,update,delete}.ts
      streams/     {open,token}.ts    <- token.ts mints the SSE token; open.ts via starter-client-react SSE hook (F5)
      me/get.ts                       <- GET /api/v1/me
    auth/                             <- thin layer over @nube/starter-ui-core useAuth
      usePrincipal.ts                 <- /api/v1/me → principal
      useCan.ts                       <- grant check
    store/
      ui.ts                           <- zustand (the ui-core singleton): edit-mode, selection
    data/
      types.ts                        <- Dashboard/Widget/WidgetConfig + datasource/query/live refs (F7)
      # NO fake.ts, NO seed.ts, NO localStorage store — F0. The mock's data/* is not ported.
    features/
      dashboards/
        Sidebar.tsx
        DashboardPage.tsx             <- split from the 177-line mock: page shell only
        DashboardToolbar.tsx          <- view/edit toggle (extracted)
        DashboardFormDialog.tsx
      canvas/
        DashboardGrid.tsx             <- react-grid-layout host
        AddWidgetDialog.tsx
      widgets/                        <- F6, ECharts
        Line.tsx  Area.tsx  Gauge.tsx  Stat.tsx  Status.tsx  DeviceTable.tsx
        WidgetCard.tsx                <- frame + per-widget data subscription
      query-editor/
        QueryEditor.tsx               <- datasource + SQL author
        Explore.tsx                   <- ad-hoc run
    extensions/
      host.tsx                        <- ExtensionHostProvider wiring + bootstrapExtensions()
      ExtensionSlot.tsx               <- re-export/host glue for <ExtensionSlot id="…">
    components/ui/                    <- thin wrappers over @nube/starter-ui-kit (F11); don't re-copy shadcn
    index.css                         <- @nube/starter-theme-tokens + Nexus accents/glass on top
```

`mod`-equivalents (`index.ts` barrels) re-export only — never hold component bodies (F1).

## Port plan — keep / swap / add (from the `nexus-ui` mock)

The mock is a **UX/visual reference** — almost nothing ports as code. What carries over is the
*look* (layout, density, accents), the widget *visual logic*, and the data-model *shape*.
Everything that touches data or framework is rebuilt on the starter platform.

| Concern | Mock today | Action in `nexus/ui` |
|---|---|---|
| React | **19** ✅ (already) | keep — singleton-pin to rubix remotes |
| Shell / CRUD | `@refinedev/core` | **remove Refine** → `@nube/starter-client-react` hooks (F2) |
| Data fetching | Refine hooks over `localStorage` | `@nube/starter-client-ts` (codegen'd) over `nexus-api` (F2) — **no localStorage, no fake (F0)** |
| Client state | component state | **zustand** via `@nube/starter-ui-core` (F3) |
| Charts | **Recharts** | **ECharts** (F6); reuse `@nube/starter-ui-dashboard` tiles where they fit |
| Canvas | react-grid-layout | keep (spike dnd-kit — maintenance mode) |
| Live data | — (fake generators) | **SSE** via `starter-client-react` hook + signed token (F5); **delete fake.ts (F0)** |
| Auth | — | `usePrincipal()`/`useCan()` over `@nube/starter-ui-core` `useAuth` + `/api/v1/me` |
| Federation | none | **host** `@nube/starter-ext-ui` + `<ExtensionSlot>`s (F3/F8) |
| Data model | `metric`-keyed fake config | **extend**: datasource+query+fields+live refs (F7) |
| Primitives / tokens | hand-copied shadcn + `index.css` | `@nube/starter-ui-kit` + `@nube/starter-theme-tokens` (F11) |
| Widget visuals | gauge math, stat/delta, status | **keep the logic**, re-skin onto ECharts / starter-ui-dashboard |
| Query editor | — | reuse `@nube/starter-ui-warehouse-explorer` (F11) |
| Teams/permissions admin | — | reuse `@nube/starter-ui-authz` (F11) |

**Carry over (logic/shape only):** widget visual logic, the grid canvas behaviour, the
data-model shape, the visual language. **Delete, never port:** `fake.ts`, `seed.ts`, the
`localStorage` store, Refine, Recharts. **Add via starter:** client + hooks, ui-kit primitives,
ui-core state/auth/i18n, federation host, query editor, authz admin.

## Phases (aligned to backend milestones)

**Parallel with backend:** P0 starts the moment the **OpenAPI contract** is published from
`nexus-spi` (see backend SCOPE "Building in parallel") — it codegens the client and builds UI
against real types before a handler exists. Phases that *verify* against data need the matching
backend milestone live, because there is no fake fallback (F0).

### P0 — Scaffold on the starter platform (starts once the OpenAPI contract is published)
- Scaffold `nexus/ui` (Vite, React 19). Wire `StarterClientProvider` + `starter-ui-core`
  `AuthProvider` + `ExtensionHostProvider`. Adopt `starter-ui-kit` + `starter-theme-tokens`; carry
  Nexus accents over `index.css`.
- **Codegen `@nube/starter-client-ts`** from the `nexus-spi` OpenAPI snapshot. Remove
  `@refinedev/*`. Build the extended `data/types.ts` (F7).
- Screens render **loading / empty / error** only — **no fake data, no `fake.ts`/`seed.ts`/
  localStorage** (F0).
- **Exit:** builds; `npm run typecheck` clean; **zero `@refinedev/*`, zero `fake.ts`/`seed.ts`/
  `localStorage`** in the tree; client is codegen'd (not hand-written); every file < 400 lines.

### P1 — Dashboards/panels CRUD against a real `nexus-api` (needs backend M1/M2)
- `api/` verb-bindings; `usePrincipal`/`useCan` over `useAuth` + `/api/v1/me`; sidebar +
  dashboard/panel CRUD over `nexus-api`.
- **TDD:** the failing integration test (against a **real** `nexus-api` — testcontainers/dev) is
  written first (F10).
- **Exit:** CRUD runs against a real tenant-scoped backend; unauthorized panels hidden by
  `useCan()`; integration tests green against the real backend.

### P2 — Query editor + ECharts panels over `POST /query` (needs backend M0/M2)
- Reuse `@nube/starter-ui-warehouse-explorer` for the editor/Explore; ECharts panels render from
  real `POST /query` JSON. **Resolve the `starter-ui-dashboard` vs ECharts adoption decision here.**
- **Exit:** a panel renders **real rows** from a datasource query; the editor runs ad-hoc SQL;
  zero fabricated series anywhere.

### P3 — Live panels via SSE (needs backend M0.5/M3)
- `api/streams/{token,open}.ts` using the `starter-client-react` SSE hook + signed token (F5);
  reconnect/heartbeat.
- **Exit:** a live panel ticks from a **real** stream without a Bearer header; clean unsubscribe.

### P4 — Federation host + admin (by backend M2 — rubix compat is a hard requirement)
- `extensions/host.tsx` + `<ExtensionSlot>`s; mount `com.nubeio.ce` unchanged; CSS scoping (F8).
  Teams/permissions admin via `@nube/starter-ui-authz`.
- **Exit:** the rubix devices/wiresheet/nav-tree remote renders inside nexus/ui with zero changes;
  one React, one QueryClient, one zustand instance; team/grant admin works against the backend.

### P5 — OpenUI "Ask Nexus" (deferred)
- `@nube/starter-ui-ai-builder` + the widget library as OpenUI components; stream a `Dashboard`
  from Claude via `nexus-api`; persist through the same client. No work now — just don't break
  F6/F7.

## Smoke tests

- **"No mock data" (F0):** `grep -rE 'fake|seed|localStorage|msw' src/` finds nothing in app
  code; with the backend down, screens show empty/error states — never invented rows.
- **"One component per file" (F1):** every `.tsx`/`.ts` < 400 lines; no `utils`/`helpers`/body-`index`.
- **"Starter client, not fetch" (F2):** no raw `fetch(` in `src/`; the client is codegen'd from
  OpenAPI; wire types are not hand-edited.
- **"Single instances" (F3):** no duplicate-React hook errors; one `QueryClient`, one zustand store
  across host + remote.
- **"rubix mounts unchanged" (F4/F8):** `com.nubeio.ce` renders via `<ExtensionSlot>` with no edits
  to its source; its CSS stays scoped under `data-ext-id`.
- **"Live without Bearer" (F5):** a live panel authenticates via token/cookie; no `Authorization`
  header on the SSE request.
- **"Panels are real" (F6/F7):** widgets render from `nexus-api` JSON; `data/types.ts` has zero
  React imports and carries datasource+query refs.
- **"Test-first" (F10):** every component/hook has a co-located test; integration tests run
  against a real `nexus-api`, not a faked network.
- **"Primitives from starter" (F11):** UI primitives import from `@nube/starter-ui-kit`, not a
  re-copied `shadcn`.

## Subagent work-units (parallelizable)

| Unit | Files | Depends on |
|---|---|---|
| **W1 scaffold + platform wiring** | `index.html`, `vite.config.ts`, `package.json`, `index.css` (theme-tokens + accents), `app/{providers,router}.tsx`, `main.tsx`; adopt `starter-ui-kit` | — |
| **W2 data model** | `data/types.ts` (extended, F7) | — |
| **W3 codegen client + dashboards/panels bindings** | `api/client.ts` (configure `starter-client-ts`), `api/{dashboards,panels}/**` | W2 + published OpenAPI |
| **W4 datasources + query + streams bindings** | `api/{datasources,streams}/**`, `api/me/get.ts` | W3 |
| **W5 auth bindings** | `auth/{usePrincipal,useCan}.ts` over `ui-core` `useAuth` | W4 |
| **W6 zustand store** | `store/ui.ts` (ui-core singleton) | — |
| **W7 dashboards feature** | `features/dashboards/**` | W3,W6 |
| **W8 canvas** | `features/canvas/**` | W6,W7 |
| **W9 ECharts widgets** | `features/widgets/**` | W2,W4 |
| **W10 query editor (reuse warehouse-explorer)** | `features/query-editor/**` wrapping `@nube/starter-ui-warehouse-explorer` | W4,W9 |
| **W11 federation host** | `extensions/{host,ExtensionSlot}.tsx` | W1,W6 |

W1 + W2 unblock everything — dispatch them first; the codegen client (W3) waits on the published
OpenAPI snapshot. Each unit is **test-first** and ships its co-located test (F10).

## Non-goals (v1)

- **No mock/fake/seed/localStorage data. Absolute (F0).** Empty/loading/error states instead.
- **No hand-rolled data layer or re-copied shadcn** — `starter-client-ts/-react` + `starter-ui-kit`
  (F2/F11).
- **No TanStack Router** — routing is host-only React Router (F4).
- **No webpack/Rspack Module Federation** — custom Vite library-mode SDK federation only
  (`@nube/starter-ext-ui`); don't introduce `@module-federation/*`.
- **No second state library** beyond the `starter-ui-core` zustand singleton; no Redux/MobX.
- **No SSR / Next.js** — Vite SPA.
- **No fetching inside widgets** — widgets are pure (F6); data arrives via props/hooks.
- **No OpenUI work yet** — P5 is deferred; only keep the door open.

## Bottom line

**Build `nexus/ui` on the starter UI platform** — `starter-client-ts/-react` for data + auth +
SSE, `starter-ui-kit`/`-core`/`-theme-tokens` for primitives/state/tokens, `starter-ext-ui` for
the federation host, and reuse `starter-ui-warehouse-explorer` / `-authz` where they fit. The
`nexus-ui` mock is the **UX reference**, not a codebase to copy. **Zero mock data (F0)** — every
pixel comes from `nexus-api`. **Test-first (F10).** One component per file, ≤400 lines, singletons
shared, SSE not Bearer. The custom work is the canvas + the ECharts panel engine.
