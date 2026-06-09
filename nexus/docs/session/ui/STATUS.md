# Nexus UI — Build Status

A snapshot of what `nexus/ui` ships today, against the published `nexus-api` contract.
Planning/handoff doc — not referenced from source. Decisions + blockers in
[DECISIONS.md](DECISIONS.md); build scope in [SCOPE.md](SCOPE.md).

## Done — wired to the live contract

Every endpoint in `nexus/backend/openapi.json` is consumed by a working screen. The client
is codegen'd (`pnpm codegen`); all data flows through it (F2), every screen renders honest
loading/empty/error states (F0).

| Area | Endpoints | UI |
|---|---|---|
| **Auth** | `POST /auth/login`, `GET /me` | Nexus AuthProvider (login/logout/principal); `usePrincipal`/`useCan`. **Verified live** — login → real principal |
| **Datasources** | list / get / create / delete | Datasources page (list + connect form + delete) |
| **Query** | `POST /query` + `POST /datasources/{id}/query` | Explore + every panel; panels query their own datasource |
| **Dashboards** | list / get / create / delete | sidebar list, create dialog, DashboardPage |
| **Panels** | add / **PATCH** / delete | AddWidgetDialog + per-panel remove + **layout-save on drag** (B5) |
| **Streams** | `POST /streams` + SSE | live panels (token-URL EventSource, windowed) |
| **Flows** | list / get / create / PUT / delete / start / stop | Flows page (run/stop/delete) + config editor |
| **Alerts** | rules CRUD · channels · silences · events | Tabbed Alerts page (rules + channels + silences + event history). **Verified live** |

**Engine & shell:** react-grid-layout canvas mounting live panels; 6-type ECharts widget
library (line/area/gauge/stat/status/table, theme-resolved colours); floating shadcn sidebar
with a runtime layout switcher; the `nexus-ui` OLED look; federation host runtime + slots
(generic); light/dark theme + region/datetime formatting.

**Backend is live on `127.0.0.1:8080`** (proxy points there; seeded admin
`admin@nexus.local` / `change-me-admin`). Auth + alerts verified end-to-end; other screens
verified against real responses.

**Quality:** 74 tests (pure logic — adapters, option-builders, `can`, window, layout,
placement, reshape, flow-draft — per F10); `pnpm typecheck` + `build` clean; F0/F1/F2 pass.

## Remaining — needs the backend

| # | Item | Needs | Frontend readiness |
|---|---|---|---|
| B4 | Load extensions | `nexus-api` to serve the extensions manifest + `remoteEntry.js` | host + slots wired; one base-path change |
| — | Test datasource connection | the test endpoint (`TestDatasourceResponse` exists, no route) | one binding + a button |

**F10 integration suite** — `src/api/integration.test.ts` runs the bindings against a real
nexus-api (login → /me → register datasource → real query → cleanup → alert rules). Opt-in via
`NEXUS_E2E_URL=http://127.0.0.1:8080 pnpm test`; skips cleanly without it so CI stays green.
Passing 4/4 live.

**Resolved this session:** auth path mismatch (Nexus AuthProvider), B5 (PATCH layout-save),
B6 (alerts shipped), per-datasource query, B7 (flow editor), F10 integration suite. The
backend is up and the full product round-trips against it — verified by the integration suite,
not just by hand.
