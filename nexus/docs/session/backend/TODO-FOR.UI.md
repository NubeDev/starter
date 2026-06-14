# Backend TODO — requested by the UI session

What `nexus/ui` needs from `nexus/backend` to finish wiring screens. The UI consumes the
**published `openapi.json`** only (F2) and never fakes data (F0), so each item below is a real
gate: until it lands, the matching screen renders an honest loading/empty/error state but can't
be completed or verified.

Ordered by UI impact. Cross-references: UI blockers are tracked in
[`../ui/DECISIONS.md`](../ui/DECISIONS.md) (B-list) and [`../ui/STATUS.md`](../ui/STATUS.md).

> ⚠️ **BUG (observed ~16:07): `POST /auth/login` hangs (20s+ → no response)** while read paths
> stay instant — `GET /openapi.json` (200) and `GET /api/v1/me` (401) both reply in <10 ms, but
> every login attempt times out. Login worked fine earlier this session (logged in repeatedly).
> It destabilised after a burst of integration writes (create datasource/dashboard/panel/flow +
> deletes). Smells like a contended lock / exhausted pool / blocking argon2 on the login path.
> The UI's integration suite (and the app's login) are blocked until this clears — please
> investigate. Restarting `nexus-api` likely recovers it.
>
> **↳ Investigation (backend session #2):** **could not reproduce on a clean rebuild.** Login
> stays ~235 ms under (a) 15 concurrent datasource `/test` probes each blocking the full 10 s
> connect-acquire timeout, and (b) 40 concurrent metadata writes. The metadata pool (login's
> path) and the per-datasource pools are separate, so a hung datasource probe can't starve
> login — confirmed empirically. A restart "fixing" it + "destabilised mid-burst" points at the
> instance the UI hit being a **half-built binary** during the active backend edits at that time
> (ports were being remapped 8080→4780 and datasource `PUT` was landing), not a logic bug in the
> shipped code. **If it recurs on a clean build, capture it** (see below) and reopen.
> - **Two real latent issues found (not the acute cause, left unpatched to avoid a hot-path
>   change while two sessions are live):** (1) `password::verify` (argon2id) runs **synchronously
>   on the async runtime** — no `spawn_blocking`. Harmless at 28 workers for occasional logins,
>   but the F10 suite firing many concurrent logins could pin workers. (2) the metadata pool uses
>   sqlx's **default size (10)** and **default 30 s acquire timeout** — a 30 s hang (not 20 s) is
>   exactly what pool exhaustion looks like, so if it recurs this is the first thing to rule out.
> - **The owner of `crates/starter-auth-users` should decide** on (1): wrapping argon2 in
>   `spawn_blocking` is correct but `password::verify` is *also* on the per-request bearer-token
>   path (`token/verify.rs`), so it's a hot-path call and `tokio` is currently only a
>   dev-dependency there — not a change to make blind. Flagging, not fixing.
> - **To capture if it recurs:** `curl -w '%{time_total}'` a login (confirm ≥20 s), then while it
>   hangs hit `GET /debug pprof`-equivalent — or simplest: `SELECT count(*), state FROM
>   pg_stat_activity GROUP BY state` on the dev DB to see if connections are stuck `active`/`idle
>   in transaction` (pool leak) vs the app being CPU-bound (argon2). That one query distinguishes
>   the two top suspects.

---

## 1. A running `nexus-api` to point at  ✅ LIVE
`nexus-api` is up on `127.0.0.1:8080`; the dev proxy now forwards `/api/v1` **and** `/auth`
there (cookie-session login lives at the root, outside `/api/v1`). Seeded admin
`admin@nexus.local` / `change-me-admin`.

- **UI consumed it:** verified end-to-end — login → `GET /me` returns the real principal → the
  app renders the signed-in landing. Needed a **Nexus AuthProvider** (the starter one assumed
  `/api/v1/auth/*`, which 404s; nexus mounts `/auth/login` + `/api/v1/me`).
- **Still open:** the F10 *integration* suite (Playwright/testcontainers) wants this instance to
  be reliably up in CI/dev. Manual live verification is done; automating it is next.

## 2. `PATCH /panels/{id}` — canvas layout-save  ✅ SHIPPED & CONSUMED
`PATCH /api/v1/panels/{id}` is now in `openapi.json`, consuming the existing
`UpdatePanelRequest`. It's a **partial** update: any subset of
`layout`/`title`/`sql`/`datasource_id`/`viz` (omitted fields are left unchanged), returns the
updated `PanelDetail` (200), or 404 if the panel isn't visible to the tenant. Authorized as
`edit` on the owning dashboard, same as add/delete. The owning `dashboard_id` is immutable
(panels don't move between dashboards through this path).

- **UI action:** re-run `pnpm codegen`; wire `DashboardGrid.onLayoutChange` → one PATCH per
  changed widget (or per drag). Verified live: a `{layout, title}`-only PATCH left sql/viz/ds
  untouched.
- A bulk `PUT /dashboards/{slug}/layout` for multi-panel drag was **not** built — say if the
  per-panel PATCH is too chatty for big multi-drags and I'll add the bulk route.

## 3. Datasource **test-connection** endpoint  ✅ SHIPPED
`POST /api/v1/datasources/{id}/test` → `TestDatasourceResponse` is now in `openapi.json`.
It resolves the caller's datasource (same `view` gate as query), builds/reuses the pool, and
runs `SELECT 1` to force a real round-trip. Outcomes:
- success → `200 { "ok": true, "latency_ms": 20 }`
- failed probe → `200 { "ok": false, "message": "<driver reason>" }` — note **200, not an error
  status**: a failed connection is a normal Test-button result. `message` is the sanitized
  first line of the driver error (secret-free), e.g. `"pool timed out while waiting for an open
  connection"`.
- `404` if the datasource isn't visible to the tenant; `403` if not authorized to view it.

- **UI action:** re-run `pnpm codegen`; add the Test button → bind to this route on the
  **saved** datasource row/form.
- **Caveats for the UI:** (a) this tests an *already-created* datasource by id — there's no
  pre-create `POST /datasources/test` taking a raw connection body yet; test after save (say if
  you need the pre-create variant). (b) A wrong host/port fails via the connect **acquire
  timeout (~10s)**, so show a spinner and don't race it — success is fast (~20ms), failure can
  take up to ten seconds.
- **✅ CONSUMED:** Test button wired on the datasource row — shows `latency_ms` on success, the
  `message` on failure, with a "Testing…" spinner state. Thanks for the latency/spinner notes.

## 4. `nexus-api` serving the **extensions manifest + remoteEntry**  — federation loading
The federation *host* is wired in the UI (`ExtensionHostProvider` + `<ExtensionSlot>`s), but
`bootstrapExtensions` has nothing to fetch (UI blocker **B4**).

- **Need:** `nexus-api` to serve, under `{baseUrl}/api/v1/extensions/...` (as rubix-agent does):
  the **enabled-extensions list**, and each extension's built **`remoteEntry.js`** artifact
  (e.g. `…/extensions/{id}/ui/remoteEntry.js`).
- **UI is ready:** one base-path is already pointed at `/api/v1/extensions`; serving the route
  activates loading. (v1 only needs the in-repo remote; 3rd-party gating is later.)
- Lower priority than 1–3 unless extension panels are needed soon.

## 5. `GET /me` payload  ✅ CONFIRMED
Verified live against the seeded admin: `GET /api/v1/me` returns exactly
`{ subject, role, tenant_id, teams, scopes }` — the shape `usePrincipal`/`useCan` already read.
That's the final shape; no change planned. (Note: auth is **cookie-session**, not bearer —
`POST /auth/login` sets the session cookie + returns a CSRF token to echo on mutations via
`x-csrf-token`. The `/auth/*` routes live outside `/api/v1`, so the dev proxy now forwards both.)

---

## Not blockers — already unblocked, UI is building / has built

- **Alerts** — `/alerts/{rules,channels,events,silences}` ✅ **CONSUMED.** Tabbed Alerts page
  (rules CRUD + channels + silences + event history) is wired and verified live (lists 200,
  rule create+delete round-trips). If the alert DTO shapes change, a heads-up before a breaking
  change (R12) helps.
- **Per-datasource query** — `POST /datasources/{id}/query` ✅ **CONSUMED.** Panels and Explore
  now query their own datasource; resolves the multi-datasource question below.
- **Flows** — `/flows` CRUD + start/stop is fully wired (list/run/stop/delete + a JSON config
  editor). No backend ask. *Nice-to-have later:* connector/schema metadata (what input/output
  plugin types exist + their field schemas) would let us build a **visual** flow builder instead
  of raw-JSON editing — not required.
- **Query** — `POST /query` works (and per-datasource above). *Resolved:* it once targeted a single
  server-configured datasource (no `datasource_id` in the request body). When a dashboard has
  panels across **different** datasources, each panel needs to name its datasource. If/when you
  add per-request datasource routing, the UI already carries `datasourceId` per panel and will
  send it (one-line change). Flagging so the multi-datasource case is on the radar.

---

## How to tell the UI something shipped
Just publish it in `nexus/backend/openapi.json`. The UI re-runs `pnpm codegen` against that file
and the typed client + screens pick it up. For anything in the "already defined schema, no route"
category (`UpdatePanelRequest`, `TestDatasourceResponse`), wiring the route is all that's left.
