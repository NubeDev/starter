# Backend TODO — requested by the UI session

What `nexus/ui` needs from `nexus/backend` to finish wiring screens. The UI consumes the
**published `openapi.json`** only (F2) and never fakes data (F0), so each item below is a real
gate: until it lands, the matching screen renders an honest loading/empty/error state but can't
be completed or verified.

Ordered by UI impact. Cross-references: UI blockers are tracked in
[`../ui/DECISIONS.md`](../ui/DECISIONS.md) (B-list) and [`../ui/STATUS.md`](../ui/STATUS.md).

---

## 1. A running `nexus-api` to point at  — **highest leverage**
The single biggest unblocker. The UI's dev runtime and its **integration tests run against a
real backend** (README §6 / R11 — no mocks). Nothing is listening on a port yet.

- **Need:** a dev instance of `nexus-api` reachable on a known port (with a seeded tenant +
  login so `/me` returns a real principal).
- **UI proxies to** `127.0.0.1:8099` today (`nexus/ui/vite.config.ts`); tell us the real port and
  we'll repoint it.
- **Unblocks:** live verification of *every* screen end-to-end, and the F10 integration tests
  (which can't be written against a faked network).

## 2. `PATCH /panels/{id}` (or a bulk dashboard-layout `PUT`)  — canvas layout-save
The contract has `POST /dashboards/{slug}/panels` and `DELETE /panels/{id}` but **no panel
update**, so a drag/resize on the canvas can't be persisted (UI blocker **B5**).

- **Need:** `PATCH /panels/{id}` accepting at least `layout` (the opaque grid JSON the UI owns),
  ideally also `title`/`sql`/`datasource_id`/`viz` so panels are editable without delete+re-add.
  A bulk `PUT /dashboards/{slug}/layout` taking all panel positions at once would be even better
  for a multi-panel drag.
- **UI is ready:** `DashboardGrid.onLayoutChange` already computes the changed-widgets diff
  (`applyGridLayout`); persisting is a one-call addition the moment the route exists.
- **Note:** `UpdatePanelRequest` is *already defined* in the contract schema — it just has no
  route consuming it.

## 3. Datasource **test-connection** endpoint
`TestDatasourceResponse` is defined in the contract schema but **no route returns it**, so the
"Test connection" affordance in the datasource form has nothing to call.

- **Need:** something like `POST /datasources/{id}/test` (or a pre-create
  `POST /datasources/test` taking a connection body) → `TestDatasourceResponse`.
- **UI adds:** a Test button on the datasource form/row — one binding + a button.

## 4. `nexus-api` serving the **extensions manifest + remoteEntry**  — federation loading
The federation *host* is wired in the UI (`ExtensionHostProvider` + `<ExtensionSlot>`s), but
`bootstrapExtensions` has nothing to fetch (UI blocker **B4**).

- **Need:** `nexus-api` to serve, under `{baseUrl}/api/v1/extensions/...` (as rubix-agent does):
  the **enabled-extensions list**, and each extension's built **`remoteEntry.js`** artifact
  (e.g. `…/extensions/{id}/ui/remoteEntry.js`).
- **UI is ready:** one base-path is already pointed at `/api/v1/extensions`; serving the route
  activates loading. (v1 only needs the in-repo remote; 3rd-party gating is later.)
- Lower priority than 1–3 unless extension panels are needed soon.

## 5. `GET /me` payload — please confirm it's complete
The UI's `usePrincipal`/`useCan` read `{ subject, role, scopes, teams, tenant_id }` from `/me`.
That's what the current contract returns and it's sufficient for per-user gating — **no change
needed unless** more is required for authz (e.g. effective permissions beyond team/scope
strings). Flagging only so it's a conscious "yes, that's the final shape."

---

## Not blockers — already unblocked, UI is building / has built

- **Alerts** — `/alerts/{rules,channels,events,silences}` **just landed in the contract** ✅.
  The UI placeholder is being replaced with real screens; no backend action needed. If the
  alert DTO shapes are still in flux, a heads-up before a breaking change (R12) helps.
- **Flows** — `/flows` CRUD + start/stop is fully wired (list/run/stop/delete + a JSON config
  editor). No backend ask. *Nice-to-have later:* connector/schema metadata (what input/output
  plugin types exist + their field schemas) would let us build a **visual** flow builder instead
  of raw-JSON editing — not required.
- **Query** — `POST /query` works. *Open question, not blocking:* it currently targets a single
  server-configured datasource (no `datasource_id` in the request body). When a dashboard has
  panels across **different** datasources, each panel needs to name its datasource. If/when you
  add per-request datasource routing, the UI already carries `datasourceId` per panel and will
  send it (one-line change). Flagging so the multi-datasource case is on the radar.

---

## How to tell the UI something shipped
Just publish it in `nexus/backend/openapi.json`. The UI re-runs `pnpm codegen` against that file
and the typed client + screens pick it up. For anything in the "already defined schema, no route"
category (`UpdatePanelRequest`, `TestDatasourceResponse`), wiring the route is all that's left.
