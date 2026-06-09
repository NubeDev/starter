# Nexus UI — Decisions & Blockers

Running log of decisions made while building `nexus/ui`, and anything blocked on the
backend. Planning doc — not referenced from source (README §3).

## Resolved decisions

### D1 — `starter-ui-dashboard` vs ECharts (SCOPE open decision, OQ at P2)
**Decision: ECharts owns the dense panels; `starter-ui-dashboard` tiles are reused for KPI/stat/
status presentation only.**

Why: `@nube/starter-ui-dashboard` is pure-presentation (data via props, zero I/O — good for F6)
but renders with **Motion/hand-rolled SVG**, not a charting engine. It has no line/area/gauge with
axes, zoom, or many-point series. For Nexus's dense time-series panels that is the wrong tool —
ECharts (canvas, downsampling, live re-render) is required by SCOPE F6 anyway. So:

- **ECharts** — `Line`, `Area`, `Gauge` (and any dense series panel). Built in `features/widgets/`.
- **`starter-ui-dashboard`** — candidate for `Stat`/KPI and `Status`/activity tiles where its
  `MetricCard` / `RadialProgress` / `ActivityFeed` fit. Reused *through* our `Widget` contract: our
  widget component maps fetched DTO → the tile's props. We still own the file (one per widget, F1).
- The mock's gauge *threshold math* (ascending vs descending thresholds, e.g. load vs battery SoC)
  is carried into the ECharts `Gauge` — it's pure visual logic, not fake data.

### D2 — Query editor: build, don't reuse `starter-ui-warehouse-explorer`
SCOPE listed `@nube/starter-ui-warehouse-explorer` as a strong reuse candidate for the query
editor/Explore. **It is reference-only, not reusable**, because it is hard-bound to the
`/api/warehouse/explorer/*` endpoints and fetches internally — it cannot drive Nexus's
`POST /query`. We keep its *patterns* (Monaco SQL editor wrapper + `react-data-grid` results grid)
and build `features/query-editor/` against the codegen'd Nexus client. Revisit only if the
warehouse-explorer gains a datasource/endpoint injection prop.

### D3 — Providers: use the starter platform stack verbatim from `rubix/frontend`
The canonical host wiring is `rubix/frontend/src/main.tsx`. We mirror its provider nesting
(`QueryClientProvider` → `StarterClientProvider` → `ExtensionHostProvider` → ui-core layout/theme/
i18n → `AuthProvider`). Differences for Nexus: **React Router (host-only, F4)** instead of TanStack
Router (rubix uses TanStack Router; SCOPE F4 mandates React Router for Nexus, and federation does
not require a router choice). No `RubixClientProvider`/SDUI providers in v1.

### D4 — SSE via `useEventStream`, not a hand-rolled EventSource (F5)
`@nube/starter-client-react` ships `useEventStream(path, opts)` which uses `EventSource` with
`withCredentials` (cookie auth) and a fetch fallback — exactly F5 (no Bearer header). Live panels
use it; we do **not** write our own EventSource. The signed-stream-token mint (`api/streams/token`)
is only needed if the deployment uses token-in-URL rather than the cookie session; wire it when the
backend stream auth model is known (see B3).

### D5 — Singletons published on `globalThis` before any remote import
`rubix/frontend/src/main.tsx` publishes React/react-dom/jsx-runtime onto `globalThis.__rubixReact*`
before any extension `import()`, because the rubix remotes' importmap shims read those globals. To
mount `com.nubeio.ce` unchanged we must replicate that publishing step and the `index.html`
importmap. (W11.)

### D6 — Visual identity is `nexus-ui`, not the starter-kit default look
**Components from `@nube/starter-ui-kit` (shadcn primitives — keeps federation + a11y);
appearance from `nexus-ui`'s `index.css` (OLED palette, emerald accent, glass, aurora,
type, density), ported faithfully and then elevated with the `ui-ux-pro` skill.**

Why: F11 says reuse the kit's *primitives* — that stands (Button/Card/Dialog must be the
shared shadcn components so federation and accessibility work). But the kit's default
*token values* are a generic look; the product's identity is the `nexus-ui` mock. So we
keep the kit's component layer and override every visual token (`--background`,
`--primary`, radius, fonts, the glass/aurora utilities) with the `nexus-ui` design. The
kit reads CSS vars, so overriding the vars re-skins every primitive without forking them.
This supersedes the earlier "keep Nexus accents on top of kit oklch" framing — the
`nexus-ui` look is the source of truth for appearance, not an accent layer.

### D7 — Extensions are generic; no `com.nubeio.ce` coupling in v1
Product owner: Nexus needs a **generic** extension host (any extension can contribute to
named slots), **not** the rubix devices/wiresheet/nav-tree remote mounted specifically.
So the federation host runtime + `<ExtensionSlot>`s stay (W11), but nexus/ui does not
depend on or hard-wire `com.nubeio.ce`. The shared-TanStack-Query-singleton requirement
(and therefore "no Refine", F2) still holds — *generic* extensions need the shared cache
too. The importmap shims keep the `__rubix*` global names only because that's the SDK's
published contract; renaming is a separate SDK change, out of scope here.

### D8 — Authz admin route deferred (dependency type error)
Reusing `@nube/starter-ui-authz`'s `<AuthzAdmin />` is still the plan for the teams/pages
admin, but mounting it pulls that package's source into nexus/ui's `tsc` graph, which
currently fails `noUnusedLocals` on an unused `TenantRail` import in
`starter-ui-authz/src/panels/authz-admin.tsx`. Rather than relax our lint or patch another
session's package mid-flight, the admin route is deferred. Re-add `<AuthzAdmin />` once
that package's unused import is cleaned up (one-line fix, owned by the authz package), or
gate dependency types out of our build. Tracked, not dropped.

### D9 — Use the canonical shadcn `Sidebar`, vendored, not the kit's slim one
Product owner wants the **floating / rounded / minimisable** sidebar (the shadcn-admin
look in `test-ui/`). The kit's `@nube/starter-ui-kit` sidebar is a *slimmer custom*
component with only `collapsible` — no `floating`/`inset` variants, no `SidebarTrigger`/
`SidebarInset`/`SidebarRail`, no runtime minimise. So we vendor the **full upstream shadcn
`Sidebar`** (the 728-line component, the same one `test-ui` ships) into
`nexus/ui/src/components/ui/sidebar.tsx`, rewiring its sub-imports to the kit's primitives
(`button`/`input`/`sheet`/`tooltip`/`skeleton`/`separator`) and `useIsMobile` from
`starter-ui-core/layout`. F11 still holds for the *primitives* — only the sidebar shell
itself is the upstream component, because the kit doesn't provide an equivalent. The file
exceeds the 400-line limit but is **vendored library code (FILE-LAYOUT §4 exemption)** —
not hand-authored; re-sync from upstream shadcn rather than refactor.

A `LayoutProvider` (cookie-persisted) drives `variant` (floating default) + `collapsible`
(icon default), and a `LayoutSwitcher` dropdown lets the user change both at runtime —
mirroring shadcn-admin's config drawer. The sidebar reads its own `--sidebar-*` token
family, pinned to the Nexus OLED palette so it matches the shell; the floating panel gets
a soft elevation shadow so it reads as detached against the near-black background.

### D10 — Panel layout-save is deferred: no panel/dashboard UPDATE endpoint
The contract ships `POST /dashboards`, `POST /dashboards/{slug}/panels`, `DELETE
/panels/{id}`, `DELETE /dashboards/{slug}` — create + delete, **no update**. (The
`UpdatePanelRequest`/`UpdateDashboardRequest` schemas are defined but no route consumes
them.) So a drag/resize on the canvas cannot be *persisted*: there's no `PATCH /panels/{id}`.

Rejected: "update via add-then-remove" — it would mint a new panel id on every move,
breaking live-stream subscriptions, selection, and React keys; racy and churny. Not worth
the correctness cost for a layout tweak.

Decision: **edit-mode drag/resize works in-session (the canvas state updates), but layout
changes are not saved** until the backend adds `PATCH /panels/{id}` (or a bulk
`PUT /dashboards/{slug}/layout`). Add-panel and remove-panel *are* wired (they have
endpoints). When the update route lands, `DashboardGrid.onLayoutChange` persists via it —
the pure `applyGridLayout` diff is already there. See B5.

## Backend blockers (build what we can; wire when unblocked)

> Per the session lead: do what's possible now, record blockers here rather than stalling.

- **B1 — No `nexus` OpenAPI contract published yet.** `nexus/backend/crates/nexus-spi` is mid-build
  (utoipa-based) and there is no committed `nexus/openapi.json`/snapshot. The starter-server's root
  `openapi.json` is **not** Nexus's and must not be used for codegen (F0/F2).
  *Impact:* W3/W4/W5 (codegen'd client + dashboards/panels/datasources/streams/me bindings) cannot
  be generated. *Done instead:* everything that doesn't need the wire types — scaffold, theme,
  providers, `data/types.ts`, zustand store, and ECharts widgets tested with typed `Widget`/DTO
  props (F10). The `api/` layer is stubbed to typed bindings the moment the snapshot lands.
  *Unblock:* point `starter-client-ts` codegen at the committed nexus snapshot and run it.

- **B2 — `GET /api/v1/me` payload shape unknown.** NEXUS Risk #6: `starter`'s `/auth/me` returns
  `{subject,email,role}` only; Nexus's `me` must add `tenant_id` + teams + effective permissions for
  `usePrincipal()`/`useCan()`. Until the nexus `MeResponse` is in the contract, `auth/useCan.ts`
  can't be typed against real grants. *Done instead:* `useCan`/`usePrincipal` are written against a
  small local principal interface that the codegen'd `MeResponse` will satisfy; swap the import when
  B1 lands.

- **B3 — SSE stream auth model undecided on the backend** (cookie session vs signed token in URL).
  `useEventStream` covers the cookie path today. If the backend chooses signed-token-in-URL,
  `api/streams/token.ts` mints it and `open.ts` appends it. *Done instead:* nothing wired to a live
  stream yet (needs M0.5); the hook choice (D4) is final.

- **B4 — `com.nubeio.ce` remoteEntry served by `nexus-api`.** The host loads remotes from
  `{baseUrl}/api/v1/extensions/...`. `nexus-api` must serve the extensions list + the
  `com.nubeio.ce/ui/remoteEntry.js` artifact (as `rubix` does). Until nexus-api serves that route,
  `bootstrapExtensions` has nothing to load. *Done instead:* the host runtime + `<ExtensionSlot>`s
  are wired so a single env/base-path change activates loading once the route exists.

- **B5 — no panel/dashboard UPDATE endpoint** (see D10). Canvas drag/resize can't persist
  without `PATCH /panels/{id}` (or a bulk dashboard-layout PUT). *Done instead:* add-panel and
  remove-panel UI are fully wired (those endpoints exist); edit-mode drag works in-session;
  `DashboardGrid.onLayoutChange` already computes the changed-widgets diff (`applyGridLayout`)
  so persistence is a one-call addition when the route lands. The `viz`/`sql`/`datasource`/
  `title` of an existing panel are likewise non-editable until update exists — edits today are
  remove + re-add (acceptable for those fields; not for layout, which moves constantly).
