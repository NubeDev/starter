# nexus-ui — Frontend Overview

The React frontend for Nexus: a premium, dark-mode **IoT / observability dashboard
builder**. Users CRUD dashboard "pages" from the sidebar, and each page is a
drag-and-drop grid of widgets (line, area, gauge, stat/KPI, status list, device
table) on live-feeling telemetry.

> **Scope of this doc:** the `nexus-ui` app only. For the whole platform (ArkFlow
> engine, `nexus-api` control plane, auth/teams), see [`../NEXUS.md`](../NEXUS.md).
>
> **Status:** working **mockup / prototype**. Per `NEXUS.md` §7 this is currently a
> *shadcn-layout reference*, not the locked running stack — see [§6](#6-current-vs-target-stack).

---

## 1. What it does

- **Dashboard CRUD** from the sidebar — create / edit (name, icon, accent,
  description) / star / delete. Pages appear in the sidebar instantly.
- **Drag-and-drop builder** — **Edit** mode → drag a widget header to move, pull the
  corner to resize, **Add widget**, duplicate, remove. Layout auto-saves. (Edit-mode
  only; view mode locks the grid.)
- **6 widget types** — line, area, gauge (threshold-aware: nominal/warn/crit),
  stat/KPI w/ sparkline + delta, status list, device table.
- **No backend** — everything runs client-side on deterministic fake data.

---

## 2. Architecture

```
src/
├── main.tsx                     # app root: provider + router + routes + <Toaster>
├── index.css                    # theme tokens, glass utils, react-grid-layout styles
│
├── providers/
│   ├── store.ts                 # localStorage source of truth (+ seed on first run)
│   ├── dataProvider.ts          # CRUD adapter over the store  ← swap for a real API
│   └── useStore.ts              # useSyncExternalStore hooks (reactive reads)
│
├── data/
│   ├── types.ts                 # Dashboard / Widget / WidgetConfig
│   ├── seed.ts                  # 4 example dashboards
│   └── fake.ts                  # deterministic, live-feeling telemetry generators
│
├── components/
│   ├── layout/Layout.tsx        # sidebar + topbar + routed <Outlet>
│   ├── layout/Sidebar.tsx       # dashboard list + CRUD
│   ├── DashboardGrid.tsx        # react-grid-layout canvas
│   ├── AddWidgetDialog.tsx      # widget picker
│   ├── DashboardFormDialog.tsx  # create/edit dashboard
│   ├── widgets/                 # Charts, Gauge, Stat, Status, DeviceTable, WidgetCard
│   └── ui/                      # shadcn primitives (button, card, dialog, …)
│
└── pages/
    ├── Index.tsx                # redirect to first dashboard
    └── DashboardPage.tsx        # view/edit toolbar + canvas
```

### Data flow & the core split

```
Sidebar / DashboardPage  ──►  dataProvider  ──►  store  ──►  localStorage
        ▲                                          │
        └──────── useSyncExternalStore ◄───────────┘   (reactive re-render)
```

- **The CRUD layer owns the dashboard *records*** (create/edit/star/delete).
- **react-grid-layout owns the *canvas*** (widget positions/sizes within a page).
- Widget moves write straight to `store.setWidgets`, keeping the grid snappy.

This is the deliberate split echoed in `NEXUS.md`: **shell = CRUD, custom code = the
dashboard engine.** Widgets are schema-described and side-effect-free, which keeps the
door open for the OpenUI work in [§7](#7-future-generative-dashboards-with-openui).

---

## 3. Features in detail

| Widget | Notes |
|--------|-------|
| **Line / Area** | Recharts; gradient fills; last points jitter for a live feel |
| **Gauge** | Custom 270° SVG dial; threshold-aware colour (handles ascending *and* descending thresholds, e.g. load vs battery) |
| **Stat / KPI** | Big tabular number + delta vs last hour + sparkline |
| **Status list** | Subsystem health with pulsing online indicators |
| **Device table** | Signal bars, battery meter, status badges, last-seen |

Fake data is deterministic ([`data/fake.ts`](src/data/fake.ts)): a seeded PRNG keyed
by metric name keeps each series stable across re-renders while still looking organic.

---

## 4. Design system

Generated via the `ui-ux-pro` skill (style: **Dark Mode / OLED**):

- **Base** `hsl(222 47% 4%)` near-black; **accent** emerald `hsl(152 76% 44%)`
- **Chart series** emerald / cyan / violet / amber / rose
- **Type** Inter (UI) + JetBrains Mono (every metric uses tabular numerals)
- Glass panels over an ambient aurora backdrop; 150–300 ms transitions;
  `prefers-reduced-motion` respected.

Tokens live as CSS variables in [`src/index.css`](src/index.css); Tailwind maps them
in [`tailwind.config.js`](tailwind.config.js).

---

## 5. Notable fixes (so they don't regress)

- **`process` shim** ([`index.html`](index.html)) — react-grid-layout's drag core
  reads `process.env.NODE_ENV`, which Vite doesn't polyfill; without the shim,
  drag/resize handlers throw `process is not defined`. Inline script defines it before
  the bundle loads.
- **Grid transform conflict** — an entrance animation animating `transform` with
  `fill-mode: both` overwrote react-grid-layout's positioning transform, stacking every
  widget at (0,0). Fixed with an **opacity-only** entrance (`animate-widget-in`).

---

## 6. Current vs target stack

`NEXUS.md` §7 is the source of truth. This mock and the platform target differ:

| Concern | **Current (this mock)** | **Target (NEXUS.md §7 + Federation, [§7](#7-module-federation-readiness))** |
|---|---|---|
| React | **18** | **19** — must match rubix's shared-singleton major (see §7) |
| Shell / CRUD | `@refinedev/core` | **No Refine** — plain `fetch` + context |
| Data fetching | Refine hooks over localStorage | **TanStack Query** over `nexus-api` REST — *and a shared federation singleton* |
| Client state | component state | **zustand** — the other shared federation singleton |
| Charts | **Recharts** | **ECharts** (uPlot / AG Grid as needed) |
| Canvas | react-grid-layout | react-grid-layout (spike dnd-kit; it's in maintenance) |
| Live data | — | SSE on `GET /api/v1/streams/:id` — **native `EventSource` can't send a Bearer header**; use cookie / signed-token URL / fetch-based SSE reader (NEXUS §5.3) |

**Migration debt (NEXUS.md §7 / Risk 7 + federation):** bump React 18→19, remove Refine,
add **TanStack Query** + **zustand**, swap Recharts→ECharts, wire auth/authz context, point
the data layer at `nexus-api`. Until then, treat `nexus-ui` as layout/UX reference, not the
locked stack. The TanStack Query + zustand choices are **not optional** — they're dictated by
the Module Federation contract in §7, not just preference.

> The dashboard/widget **data model** ([`data/types.ts`](src/data/types.ts)) is stack-
> agnostic and survives the migration — only the provider/chart layers change.

---

## 7. Module Federation readiness (must-have)

**Hard requirement:** `nexus-ui` must be the **host** for the project's existing
extension system, and the existing **rubix UI extensions must keep working unchanged**.
The reference implementation already in this repo:

- **Host runtime:** [`starter-extensions/packages/starter-ext-ui`](../starter-extensions/packages/starter-ext-ui) (`@nube/starter-ext-ui`)
- **Extension SDK:** [`starter-extensions/packages/starter-ext-sdk-ts`](../starter-extensions/packages/starter-ext-sdk-ts) (`@nube/starter-ext-sdk-ts`)
- **Live example remote:** [`rubix/extensions/com.nubeio.ce/ui-src`](../rubix/extensions/com.nubeio.ce/ui-src) (devices panel + wiresheet canvas + nav tree)

> ⚠️ This is **not** webpack / Rspack Module Federation. It's a **custom Vite
> library-mode SDK federation**: each extension builds a single ESM `remoteEntry.js`,
> React is shared via the host's importmap, and components are contributed to **named
> slots** (not routes). Match *this* model — don't introduce `@module-federation/*`.

### The contract `nexus-ui` must implement

**1. Shared singletons (matching-majors, hard-refusal on mismatch).** The host registers
these; every remote declares the same set with a version. From `@nube/starter-ext-ui`:

| Singleton | Why it's shared |
|---|---|
| `react`, `react-dom` | one React instance — multiple copies break hooks/context |
| **`@tanstack/react-query`** | one `QueryClient` / cache shared host ↔ extensions |
| **`zustand`** | one shared client-state store instance |
| ui-core **i18n** + **preferences** | host-owned locale/format/theme, read by extensions |

> This is the answer to *"do we need TanStack?"* — **yes, TanStack Query**, because it's
> one of the four shared singletons. **Refine cannot be the data layer** (it brings its own
> query runtime that extensions can't share). **zustand** is the companion state singleton.
> Note: it's TanStack **Query**, *not* TanStack **Router** — routing stays host-only (see #3).

**2. Host provider + slots.** Wrap the app and expose mount points:

```tsx
import { ExtensionHostProvider, ExtensionSlot, bootstrapExtensions } from "@nube/starter-ext-ui";

// at startup: negotiate singletons, run each remote's init(handle)
await bootstrapExtensions({ /* StarterClient, manifest, singleton provisions */ });

<ExtensionHostProvider>
  <Sidebar>
    <ExtensionSlot id="sidebar" />        {/* extensions add nav here */}
  </Sidebar>
  <DashboardPanel>
    <ExtensionSlot id="panel" />          {/* extension-contributed panel types */}
  </DashboardPanel>
</ExtensionHostProvider>
```

`<ExtensionSlot id="…">` looks up every `contributes.ui.exposes[*]` whose `slot` matches,
wraps each in a `SlotContextProvider`, and mounts in manifest order.

**3. Routing stays in the host.** Extensions are **slot contributions keyed by name**
(`Main`, `NavTree`, …) matched via `block.yaml` — they do **not** own routes. So React
Router (current) is fine; **TanStack Router is *not* required** by federation. Keep the
router a host-only concern.

**4. Host bindings the example remote already expects.** `com.nubeio.ce` consumes the host
via `@nube/starter-ext-sdk-ts`: `useHostClient` (no raw `fetch`), `useHostTheme`,
`useHostBindings` (prefs/i18n/formatters), `BlockShell`, `useSlotContext`. `nexus-ui` must
**provide** these (theme tokens, preferences, IntlShape, a `StarterClient`) so the extension
mounts unchanged.

**5. CSS isolation.** Extension CSS is injected-by-JS and **scoped to `[data-ext-id="…"]`**
(postcss plugin in the remote's `vite.config.ts`). The host must render each extension's
subtree under a `data-ext-id` wrapper and not rely on global Tailwind bleeding in.

**6. Extension security — loading `remoteEntry.js` runs trusted code in the user's session.**
The items above are *integration*; this is *security*, and it gates loading any remote we don't
author. The host must define:
- a **manifest allowlist** (only declared remotes load) + **checksum-pin or signature** on each
  `remoteEntry.js` (defeat tampered/swapped bundles);
- a **CSP** that constrains what a remote can fetch/connect to;
- a **version/compat policy** for the singleton majors (today: hard-refuse on mismatch);
- an explicit **capability boundary on `StarterClient`** — what an extension may call/read, so a
  remote can't exfiltrate tokens or hit arbitrary nexus-api endpoints.

For v1 the only remote is in-repo (`com.nubeio.ce`), so this can trail the host runtime — but it
**must land before any third-party/out-of-repo extension loads** (NEXUS Risk #13).

### Remote build recipe (so a Nexus panel type can ship as an extension)

A panel type = an extension-contributed component, built exactly like
[`com.nubeio.ce/ui-src/vite.config.ts`](../rubix/extensions/com.nubeio.ce/ui-src/vite.config.ts):

- Vite **library mode**, `formats: ["es"]`, output `remoteEntry.js`, `inlineDynamicImports`
- `rollupOptions.external: ["react","react-dom","react/jsx-runtime","react-dom/client"]`
- `vite-plugin-css-injected-by-js` + the `[data-ext-id]` postcss scoping plugin
- `remoteEntry.ts` default-exports `{ singletons, init(handle) }` and calls
  `registerExtensionContributions(handle, { components: { … } })`

### What this means for the current mock

| Federation need | Mock today | Action |
|---|---|---|
| React **19** singleton | React **18** | bump (rubix remotes declare React 19) |
| Share `@tanstack/react-query` | Refine's own query layer | **drop Refine → TanStack Query** |
| Share `zustand` | none | add zustand for shared client state |
| Host the runtime | none | adopt `@nube/starter-ext-ui` host provider + `<ExtensionSlot>`s |
| CSS scoping | global Tailwind | render extensions under `data-ext-id`, scope their CSS |

> **Bottom line:** the federation contract *forces* the same migration NEXUS.md §7 already
> wants (no Refine, TanStack Query) and adds two more locks: **React 19** and **zustand**.
> Build the stack on `@nube/starter-ext-ui` from the start and the existing rubix extensions
> (`com.nubeio.ce`) drop in with zero changes.
>
> **Sequencing:** because rubix compat is a *hard requirement*, the **host runtime lands in M2**
> (NEXUS §9), not M4 — M4 is only the broader ecosystem of remotes we don't own, gated behind
> the extension security model (#6 above).

---

## 8. Future: generative dashboards with OpenUI

> **OpenUI** — https://github.com/thesysdev/openui — "the open standard for generative
> UI." An LLM streams a compact, token-efficient **OpenUI Lang**; a React renderer
> (`@openuidev/react-lang`) progressively turns it into live, interactive components.

### Why it fits Nexus

Nexus already has the two things OpenUI needs:

1. **A typed widget library** (`components/widgets/`, driven by `Widget` /
   `WidgetConfig`) — the "allowed components" OpenUI generates prompts from.
2. **A schema-first model** (a `Dashboard` is just a list of `Widget`s with layout +
   config) — trivial to emit as streamed structured output.

### The opportunity — "Ask Nexus"

A command-palette / chat surface where an operator types:

> *"Build me a cold-chain dashboard for the north warehouse — two freezer gauges, a
> temperature trend, and a door-event table."*

…and OpenUI streams a dashboard definition that we map onto our widgets and drop onto
the react-grid-layout canvas, **panel by panel as tokens arrive**.

### Integration sketch (when we build it)

| Step | OpenUI piece | Nexus piece |
|------|--------------|-------------|
| 1. Describe components | `@openuidev/react-ui` defs | wrap our 6 widget types as OpenUI components |
| 2. Generate system prompt | OpenUI prompt generation | from the `Widget`/`WidgetConfig` schema |
| 3. Stream from an LLM | OpenUI Lang (streaming) | Claude via `nexus-api` (keys stay server-side) |
| 4. Render progressively | `@openuidev/react-lang` | feed parsed widgets into `DashboardGrid` |
| 5. Persist | — | save the generated `Dashboard` through the **same data provider** |

Because generated dashboards land as ordinary `Dashboard` records, they're immediately
editable in the drag/drop builder — **AI drafts, the human refines.** OpenUI becomes
*one more way to create a dashboard*, with no separate code path.

**To stay OpenUI-ready now:** keep widgets schema-described and side-effect-free, and
keep `data/types.ts` the single source of widget shape. No work needed yet — just
don't paint ourselves out of it.

---

## 9. Run

```sh
npm install
npm run dev        # http://localhost:5273
npm run build      # production bundle
npm run typecheck  # tsc --noEmit
```

Seeded with 4 dashboards on first load; data persists in `localStorage`
(`nexus.dashboards.v1`). Clear it to re-seed.
