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
