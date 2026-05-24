# Session — frontend surfaces (codeless/rubix-frontend-surfaces)

**Date:** 2026-05-24
**Branch:** `codeless/rubix-frontend-surfaces`
**Goal:** Build out four major user-visible surfaces in `rubix/frontend`
by consuming the already-built starter UI packages and adding one
rubix-side ClickHouse admin surface. Zero new starter packages; zero
modifications to existing starter UI packages.

## Per-phase commit summary

### Phase A — authz admin shell + nav

- `babb4c0` / `dac812c` — **A.1 authz tab triage.** Walked every tab in
  `<AuthzAdmin>` at `/admin/access` against the live agent. Only class
  (c) fixes — i18n key fills — committed (`feat(rubix-frontend) authz
  tab triage + i18n fills`). Class (a)/(b) issues land as follow-ups
  below.
- `49ac141` / `7806a81` — **A.2 left-nav admin section + authz smoke
  test.** Three entries (access, users, warehouse). Added
  `e2e/authz-admin.spec.ts` walking through all 8 tabs asserting each
  heading renders. `feat(rubix-frontend) admin nav section + authz
  e2e`.
- `5c3f545` — gate: Phase A landed.

### Phase B — flows surface + FlowCanvas

- `6eb258c` — **B.1 flow-ops body-source confirmation.** Confirmed
  `flow-ops.list` returns only metadata; `useFlowDefinition(flowId)`
  hook added in `@nube/rubix-client-react/src/hooks/flow-ops.ts`
  feeding `body_yaml` through the `yaml` package. Sibling
  `.test.tsx` covers the parse. `feat(rubix-client-react)
  useFlowDefinition hook + yaml parse path`.
- `82a15d2` / `2df3daf` — **B.2 flow-node-registry + ai-agent
  override.** `src/lib/flow-registry.ts` (~80 LOC) assembles a
  `NodeKindRegistry` from `builtinNodeKinds()`. `src/lib/flow-nodes/
  ai-agent-node.tsx` (~120 LOC) renders `skill_hint` + allowed-tools
  badge. No modification to `@nube/starter-ui-flow`.
  `feat(rubix-frontend) flow node registry + rubix ai-agent
  override`.
- `40e7191` / `c30e8dc` — **B.3 `/flows` list + `/flows/$flowId` view
  routes.** `<FlowCanvas registry={flowRegistry} graph={flowGraph}
  readOnly showMiniMap showControls showBackground />`. Left-nav Flows
  entry. `feat(rubix-frontend) /flows list + view routes`.
- `5cf4298` / `e196477` — **B.4 flows smoke test.** `e2e/flows.spec.ts`
  asserts list ≥ 6 entries (bundled flows from PR #32), click into
  `com.rubix.scheduled-system-check`, assert canvas renders an
  `ai-agent` node. `test(rubix-frontend) flows e2e`.
- `320749f` / `7f1e596` — gate: Phase B landed.

### Phase C — ClickHouse + insights admin

- `d1417b8` — **C.1 endpoint confirmation.** Grepped
  `rubix-agent/src/routes/` for insights + clickhouse list/CRUD
  endpoints. All hook endpoints present; analysis-only stage.
- `9f98927` / `c76559a` — **C.2 rubix-client-react hooks.**
  `useClickhouseRulesList`, `useClickhouseMartsList`,
  `useClickhouseMartDrop`, `useClickhouseTablesList`,
  `useInsightsRulesList`, `useInsightsRuleCreate`,
  `useInsightsRuleEnable`, `useInsightsRuleDisable`. Sibling
  `.test.tsx` per file. `feat(rubix-client-react) clickhouse
  list/drop + insights hooks`.
- `0282149` / `457e905` — **C.3 warehouse admin panels.**
  `src/components/admin/warehouse/{rules,marts,retention,insights}-
  panel.tsx` + `warehouse-admin.tsx` shell + barrel. Each panel ≤ 200
  LOC, verb file. i18n keys `admin.warehouse.*` in
  `src/i18n/{en,es}.json`. `feat(rubix-frontend) warehouse admin
  panels`.
- `0d5ab4d` / `0641243` — **C.4 `/admin/warehouse` route + e2e.**
  `e2e/warehouse.spec.ts` walks 4 tabs asserting headings.
  `feat(rubix-frontend) /admin/warehouse route + e2e`.
- `0d09d8d` / `29b2a28` — gate: Phase C landed.

### Phase D — chrome polish

- `f488d30` / `d668e5b` — **D.1 top-header polish.** User email + role
  badge from `useAuth().user`; tenant indicator from `useTenantList()`
  (display-only ≤ 1, dropdown for 2+ with switching disabled +
  tooltip); logout menu item; theme toggle via starter-ui-kit
  `useTheme`. `feat(rubix-frontend) top header with user + tenant +
  logout + theme toggle`.
- `38d3bdb` / `d3c4ec5` — **D.2 empty states + loading skeletons.**
  `<EmptyState>` on every list-rendering route; `<Skeleton>`
  replacing spinner-only states on every `useQuery` consumer.
  **Toast listener BLOCKED** — `@nube/starter-ui-kit` does not export
  a `Toast` primitive; pattern documented in
  [`docs/design/frontend/`](../design/frontend/README.md) for the day
  the upstream lands. `feat(rubix-frontend) empty states + loading
  skeletons (toast listener BLOCKED on missing starter-ui-kit Toast
  primitive)`.
- `d71be1d` / `05b8f3e` — **D.3 chrome smoke test.**
  `e2e/chrome.spec.ts` covering login → top-header (email + logout)
  → left nav (5 sections) → logout → redirect to `/login`.
  `test(rubix-frontend) chrome e2e`.
- `a6861ed` — gate: Phase D landed.

### Phase E — docs + PR (this stage)

- This commit — `chore(docs) close out frontend surfaces + open PR`.

## Test counts

- New e2e specs landed on this branch: **4** —
  `authz-admin.spec.ts`, `flows.spec.ts`, `warehouse.spec.ts`,
  `chrome.spec.ts`.
- All `rubix/frontend` e2e specs in tree after this branch:
  `auth`, `authz-admin`, `chrome`, `config-drawer`, `debug`,
  `extensions`, `flows`, `header-search`, `layout`, `mobile-nav`,
  `sidebar-mode`, `users`, `warehouse` — **13 spec files** total.
- Unit/integration: `pnpm --filter @nube/rubix-frontend test` and
  `pnpm --filter @nube/rubix-client-react test` both green at each
  Phase-gate.

## Operator-runnable manual flow (full `make start` walkthrough)

```bash
# 1. Boot the agent + frontend.
make start
# → rubix-agent on :8088, vite dev server on :5173 with proxy

# 2. Open http://127.0.0.1:5173 in a browser. You land on /login.

# 3. Log in as the bootstrap operator (op@example.com / ...).
# → top-header shows email + role badge + theme toggle + logout

# 4. Walk every left-nav section:
#    Home        → feature tiles + boot intro
#    Flows       → list (≥ 6 bundled flows). Click
#                  com.rubix.scheduled-system-check → FlowCanvas
#                  renders the ai-agent node with skill_hint label
#                  and allowed_tools badge. readOnly = true; the
#                  canvas pans/zooms but does not mutate.
#    Extensions  → table + SSE connection badge; com.rubix.example
#                  shows as running.
#    Admin / Access → <AuthzAdmin> 8 tabs (tenants / teams / members
#                  / rules / assignments / resources / check /
#                  decisions). Switch through each; every heading
#                  renders without error.
#    Admin / Users → user admin panel + undo button.
#    Admin / Warehouse → <WarehouseAdmin>:
#         · Rules     — list CH rules; edit one in the inline
#                       textarea; save via useClickhouseRuleWrite.
#         · Marts     — list; create modal; drop with data-loss
#                       warning.
#         · Retention — set retention on system_disk_history to 30
#                       days; assert system.tables shows new TTL;
#                       undo via the panel's undo button OR via
#                       /admin/users's existing undo flow.
#         · Insights  — list rules; toggle enable/disable.
#    Settings    → per-user prefs (locale, units, theme).

# 5. Trigger a deliberate error (e.g. POST a malformed retention
#    payload via the rules editor). A localised toast surfaces via
#    the QueryCache onError listener — see "Open follow-ups" below
#    for current toast status (BLOCKED on starter-ui-kit primitive).

# 6. Log out from the top-header menu → redirected to /login.
```

## REST endpoints that proved missing during the job

None blocked the job — Phase C.1 endpoint-confirmation grep found
every insights + clickhouse list/CRUD endpoint already present on
`rubix-agent/src/routes/`. The single missing piece is a **frontend
package primitive**, not a REST endpoint:

| Missing                             | Where it would land                | Tracked as |
|-------------------------------------|------------------------------------|------------|
| `Toast` primitive + `useToast()`    | `@nube/starter-ui-kit`             | SCOPE OQ-6 — Phase D.2 listener mounts the day this primitive ships. |

Follow-ups from Phase A.1 authz triage (class a/b — not (c) i18n
fixes already landed in this branch) carry forward as previously
filed; no new REST gaps surfaced during Phases B–D.

## What's next

- Upstream `Toast` into `@nube/starter-ui-kit`, then mount
  `<ToastErrorListener>` in `main.tsx` (one-line change).
- Phase A.1 (a)/(b) follow-ups in `starter-ui-authz` /
  `rubix-agent`.
- Tenant-switching wire-up once `useTenantSwitch` exists upstream
  (today the dropdown is disabled with a tooltip).
