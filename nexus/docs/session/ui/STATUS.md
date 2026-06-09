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
| **Auth** | `GET /me` | principal landing; `usePrincipal` / `useCan` (role/scope/team gate) |
| **Datasources** | list / get / create / delete | Datasources page (list + connect form + delete) |
| **Query** | `POST /query` | Explore (datasource picker → SQL → results) + every panel |
| **Dashboards** | list / get / create / delete | sidebar list, create dialog, DashboardPage |
| **Panels** | add / delete | AddWidgetDialog + per-panel remove (edit mode) |
| **Streams** | `POST /streams` + SSE | live panels (token-URL EventSource, windowed) |
| **Flows** | list / get / create / **PUT** / delete / start / stop | Flows page (run state + start/stop + delete) |

**Engine & shell:** react-grid-layout canvas mounting live panels; 6-type ECharts widget
library (line/area/gauge/stat/status/table, theme-resolved colours); floating shadcn sidebar
with a runtime layout switcher; the `nexus-ui` OLED look; federation host runtime + slots
(generic); light/dark theme + region/datetime formatting.

**Quality:** 65 tests (pure logic — adapters, option-builders, `can`, window, layout,
placement, reshape — per F10); `pnpm typecheck` + `build` clean; F0/F1/F2 smoke tests pass.

## Blocked — needs the backend (see DECISIONS B-list)

| # | Blocked feature | Needs | Frontend readiness |
|---|---|---|---|
| B4 | Load extensions | `/extensions` route in nexus-api | host + slots wired; one base-path change |
| B5 | Persist canvas layout | `PATCH /panels` (or layout PUT) | `applyGridLayout` diff already computed |
| B6 | Alerts | `/alerts` in the contract | placeholder page + route in place |
| B7 | Flow config editor | (none — UI work) | CRUD wired; needs a pipeline-builder UI |
| — | Test datasource connection | the test endpoint (`TestDatasourceResponse` exists, no route) | one binding + a button |
| — | Live verification + integration tests (F10) | a **running** nexus-api on a port | screens render honest states until then |

## To unblock
1. **Start `nexus-api`** and point the dev proxy at its port (`vite.config.ts`, currently
   `127.0.0.1:8099`) → verify the full round-trip and write the integration tests.
2. **Publish the missing endpoints** (`PATCH /panels`, `/extensions`, `/alerts`, datasource
   test) → each is a small, well-scoped frontend addition; the engine underneath is done.
