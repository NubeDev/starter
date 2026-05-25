# 2026-05-25 — Goal 1 (Dashboards) landed end-to-end

Branch `codeless/rubix-dashboards-goal-1` lit up SCOPE Goal 1 —
operator-authored *and* AI-authored dashboards via the SDUI
substrate — in five phases (A–E) following the dependency graph
in `docs/scope/dashboards/README.md`.

## Phase-by-phase commit summary

### Phase A — storage + bindings substrate

- **A.1** `feat(rubix-store-postgres+rubix-spi) dashboards_definitions + PgDashboardStore + authz registration` (`2fb4d58`)
  - `dashboards_definitions` PG table with revisions + `pg_notify` trigger.
  - `DashboardStore` trait + `PgDashboardStore` (`insert_revision`, `get_active`, `list_active`, `mark_superseded`, `history`).
  - `boot/dashboards_seed.rs` idempotent seeder mirroring `boot/flows_seed.rs`.
  - Integration test `rubix-agent/tests/dashboards_definitions_test.rs`.
- **A.2** Six substrate gaps in `starter-ui-bindings` / `starter-ui-ir` landed as seven bisect-clean commits (`b7edc84`, `d560d68`, `10b8b83`, `f122f54`, `46b6fa2`, `817f456`, `77bd5ca`): per-variant `Bindable` trait, `substitute_tree` dispatch, qualifier grammar (`$target?` / `$target!`), `Repeat` `$item` cursor, synthetic ids, portable-subset flag, `$msg` source with `MessageBag` on `EvalCtx`.

### Phase B — host glue

- **B.1** `feat(rubix-agent) SDUI host glue — four trait impls` (`d9ddafe`)
  - `RubixEntityGraph`, `PgPageProvider`, `RubixQueryEngine` (kind-only RSQL for v1), `RubixHandlerRegistry`. Sibling tests under `rubix-agent/tests/sdui_*_test.rs`.
- **B.2** `feat(rubix-agent) mount SDUI router under /api/v1/ui` (`5fab9d9`)
  - `boot/sdui.rs` builds the router from the four trait impls and merges into `main.rs` alongside `extensions` + `flow_events`.

### Phase C — tool bodies

- **C.1** `feat(rubix-tools+rubix-spi) dashboard.get + dashboard.list` (`0bd48c0`).
- **C.2** `feat(rubix-tools+rubix-spi) dashboard.create + dashboard.update` (`3a632b8`) — reversible via `starter-undo`, conflict on stale `expected_revision_id`.
- **C.3** `feat(rubix-tools+rubix-spi) dashboard.delete + dashboard.duplicate` (`100d6e1`) — reversible, refuses delete on `created_by=system`.
- **C.4** `feat(rubix-tools+rubix-spi) dashboard.page_set runtime slot write` (`71e654a`) — runtime slot mutation via the same chokepoint flows use; not reversible (operator reverts by setting slot back).
- **C.5** `test(rubix-agent) dashboard CRUD + page_set end-to-end` (`cd4a18f`) — seven verbs wired into `rubix-agent/src/registry.rs` so they auto-surface as MCP tools per R7; `dashboard_crud_test.rs` integration test under testcontainers PG.

### Phase D — frontend + AI builder

- **D.1** `feat(starter-ui-sdui-react) new package — SduiPage + per-variant renderers + transport seam` (`9988112`).
- **D.2** `feat(rubix-frontend+rubix-flows+rubix-client-ts+rubix-client-react) dashboard-assistant flow + /dashboards routes + bundled disk-overview + e2e` (`0487da0`) — real `com.rubix.dashboard-assistant` flow, bundled `disk-overview.json`, `/dashboards` + `/dashboards/$pageId` routes, Playwright spec `rubix-frontend/e2e/dashboards.spec.ts`.

### Phase E — promotion + close-out

- This commit. Scope files 01–06 promoted to `docs/design/sdui/<area>/README.md`; scope-side replaced with one-line redirects; `08-open-questions.md` emptied (Q1–Q10 all folded into the design docs); `07-fetch-plan.md` preserved as v2 hand-off; `docs/design/sdui/README.md` cross-links the sub-docs; THIN-SLICE Goal 1 row flipped to **real**.

## Test counts

- `cargo test` total across all phases: ≈ 870 tests passing (workspace
  baseline before this branch was ≈ 720; this branch added ≈ 150
  across `starter-ui-ir`, `starter-ui-bindings`,
  `rubix-store-postgres`, `rubix-spi`, `rubix-tools`, `rubix-agent`
  + sibling tests per verb).
- `pnpm typecheck` + `pnpm test`: green workspace-wide.
- Playwright e2e: 1 new spec (`rubix-frontend/e2e/dashboards.spec.ts`) — login → `/dashboards/disk-overview` → kpi numeric assert → chart renders → time-range toggle → `page_set` lands.
- Testcontainers-gated integration tests:
  `dashboards_definitions_test.rs`, `dashboard_crud_test.rs` —
  both green when run with `RUBIX_TESTCONTAINERS=1`.

## Operator-runnable manual flow

```
make start
# log in as the seeded operator
open http://localhost:5173/dashboards
# disk-overview is listed (seeded by boot/dashboards_seed.rs)
click disk-overview
# see live KPI + chart driven by the bindings substrate +
# RubixQueryEngine

# now drive the AI builder from Claude Desktop (or curl):
#  "make me a page for cpu usage"
# the dashboard-assistant flow's ai-agent node picks
# com.rubix.dashboard-builder, calls rubix.dashboard.create with
# a pruned-schema body, returns the new page_id

open http://localhost:5173/dashboards
# new page appears in the list under the operator's principal
click the new page → it renders via <SduiPage page_ref={pageId}>

# undo:
curl -XPOST /api/v1/tools/rubix.undo.last
# the new page is superseded; reload list — it's gone
```

## Pointers

- Design: [`docs/design/sdui/`](../design/sdui/) (sub-docs:
  storage / bindings / host-glue / tools / renderer / ai-builder).
- v2 hand-off: [`docs/scope/dashboards/07-fetch-plan.md`](../scope/dashboards/07-fetch-plan.md).
- PR: opened off `codeless/rubix-dashboards-goal-1` against
  `master`, reviewed phase-by-phase via the commit graph above.
