# Scope — rubix-frontend-surfaces

## Goal

Build out the four major user-visible surfaces in `rubix/frontend` by consuming the already-built starter UI packages and adding one rubix-side warehouse admin panel set. After this job, an operator who runs `make start` from `rubix/` can: (A) administer authz fully (tenants/teams/members/rules/assignments/decisions/check tabs) via the existing `<AuthzAdmin>` shell, (B) browse and view flow definitions with the `FlowCanvas` rendering the YAML body in a visual graph, (C) administer ClickHouse rules / marts / retention / insights rules in a rubix-flavoured admin surface, and (D) navigate the app via a polished top-header showing the current user/tenant, with proper empty states, loading skeletons, and `RubixError` toasts wired to the global error boundary.

This is **pure consumption**, with one small addition. All UI primitives we need exist:

- `<AuthzAdmin>` + 8 panels (tenants/members/teams/rules/assignments/resources/check/decisions) in `@nube/starter-ui-authz` ✅
- `<FlowCanvas>` + `NodeKindRegistry` + 4 built-in node kinds in `@nube/starter-ui-flow` ✅
- 13 starter UI packages total (kit, dashboard, chat, skills, blobs, export, ai-builder, core, sdui-react …) all built and tested
- `rubix-client-react` hooks for every endpoint family (audit, clickhouse, extensions, flow-ops, mcp, system, teams, tenants, undo, users) ✅
- Auth + SSE + error boundary + i18n wired from PR #35 ✅

What's missing is **routes that mount these primitives** plus a rubix-side warehouse admin UI (since ClickHouse rules/marts/retention are rubix-specific Goal 4 surface, not starter-generic — they don't belong in an upstream package).

The success bar: a fresh contributor runs `make start`, logs in, navigates through every left-nav item, sees real data, performs at least one write per surface (create a rule, deploy a flow, set a retention TTL, create a user), watches the `RubixError` toast surface a friendly localised message on a deliberate failure, and refreshes the page without losing auth state. Every surface is real.

## In scope

### Phase A — full authz admin surface

The `access.tsx` route already mounts `<AuthzAdmin>` from `@nube/starter-ui-authz` per the merged frontend-wire PR. This phase ensures every tab actually works and adds navigation polish.

- **`rubix/frontend/src/routes/admin/access.tsx`** — verify each of the 8 tabs (tenants/teams/members/rules/assignments/resources/check/decisions) renders against the live rubix-agent backend; identify any tab that throws or shows "Not implemented"; trace the cause to either (a) the rubix-agent endpoint missing/broken, (b) a `starter-ui-authz` panel calling an API method that doesn't exist on `StarterClient`, or (c) i18n keys missing. For each broken tab, file a one-paragraph diagnosis in the handover; fix only the (c) kind in this phase (i18n / styling / wiring); (a) and (b) get tracked as follow-ups in the closing session note.
- **i18n catalogue completion** — ensure every `access.shell.*` and `access.tabs.*` key the panel exports has entries in both `rubix/frontend/src/i18n/{en,es}.json`. The `useAuthzMessagesFromIntl()` shape in `access.tsx` is the contract; mirror it for every panel-internal key by walking each panel file's `useAuthzMessages()` call sites.
- **Left-nav entry** — `/admin/access` should be reachable via the layout's left nav (or top nav). If the existing layout doesn't have an admin section, add one with three entries to be ready for Phase C/D (`access`, `users` — already there from #35 — and `warehouse` which lands in Phase C). Use existing `starter-ui-kit` nav primitives; don't author new ones.
- **Decisions audit feed** — verify the `DecisionsPanel` either renders live audit rows (if `starter-client-ts` exposes the endpoint) or surfaces an honest "endpoint not wired" message. Don't ship a fake "live" panel.
- **One smoke test** — extend `rubix/frontend/e2e/` with `authz-admin.spec.ts` covering: navigate to `/admin/access`, switch through all 8 tabs, assert each tab's heading renders without error. Doesn't assert business logic — just that the shell mounts.

### Phase B — flow browser

Two routes consuming `@nube/starter-ui-flow`'s `FlowCanvas` + `NodeKindRegistry`.

- **`rubix/frontend/src/lib/flow-registry.ts`** (~80 lines) — build a `NodeKindRegistry` at app boot. Register the 4 built-in kinds from `@nube/starter-ui-flow`'s `builtins.tsx` (`ai-agent`, `tool-call`, `trigger`, `branch`) verbatim — the rubix-side YAML loader (`rubix-flows::convert.rs`) prefixes node ids with `com.rubix.*` but the **kind** stays `ai-agent` / `tool-call` etc., so the built-in components render correctly. If a flow YAML carries a kind the registry doesn't know about, the canvas's fallback renders a labelled grey box — verify that's the existing behaviour before relying on it.
- **`rubix/frontend/src/routes/flows/index.tsx`** — list view. Consumes `useFlowsList()` from `@nube/rubix-client-react/hooks/flow-ops`. Table with columns: flow id, latest revision id, last deployed at, supersession count. Click a row → navigate to `/flows/$flowId`. Empty state if no flows. Loading skeleton via `starter-ui-kit`.
- **`rubix/frontend/src/routes/flows/$flowId.tsx`** — detail view. Consumes a new hook `useFlowDefinition(flowId)` that calls a backend endpoint returning the latest revision's `body_yaml` parsed into a `FlowGraph` (the shape `@nube/starter-ui-flow` expects). **This is the integration risk:** the `flow-ops.list` endpoint that's already wired might not return the parsed body — only the metadata. If the body endpoint doesn't exist, the rubix-side hook builds the `FlowGraph` from the YAML body it has by calling a TS-side YAML parser (e.g. `yaml` npm package — already transitively present, confirm). View-only first: `readOnly={true}` on the canvas. Editing is out of scope this job.
- **Node-kind extension for `ai-agent`'s rubix-specific config** — the bundled flow YAMLs carry `config.session_policy`, `config.skill_hint`, `config.cost_cap`, `config.allowed_tools[]`. `@nube/starter-ui-flow`'s built-in `ai-agent` node shows the kind + label but doesn't render config. Register a rubix-specific override component that renders the `skill_hint` and `allowed_tools[]` count as badges on the node body. The override goes in `rubix/frontend/src/lib/flow-nodes/ai-agent-node.tsx` (~120 lines) and registers via `NodeKindRegistry::register(...)`, replacing the built-in `ai-agent` entry. **Do not modify `@nube/starter-ui-flow`** — it stays generic; rubix's overrides live in rubix.
- **Smoke test** — `rubix/frontend/e2e/flows.spec.ts`: navigate to `/flows`, assert list renders ≥ 6 entries (the 6 bundled flows from PR #32), click `com.rubix.scheduled-system-check`, assert the canvas renders at least one node labelled `ai-agent`.

### Phase C — ClickHouse + insights admin (rubix-side, not upstream)

ClickHouse rules, marts, retention, and insights rules are rubix-specific Goal 4 territory. They live entirely in rubix; no new starter package.

- **`rubix/frontend/src/components/admin/warehouse/`** (new directory, ~6 verb files):
  - `rules-panel.tsx` (~150 lines) — list CH rules via `useClickhouseRulesList()` (add this hook to `@nube/rubix-client-react` if missing — it likely is, since the original hook file is `clickhouse.ts` and exposed `useClickhouseRuleWrite/MartCreate/RetentionSet` mutations only). Inline editor for the SQL body (use a plain `<textarea>` first; a code editor is a follow-up). Save button → `useClickhouseRuleWrite()` mutation. Delete button → soft delete via mark-disabled (matches the goals-2-4-3 undo pattern).
  - `marts-panel.tsx` (~120 lines) — list marts via `useClickhouseMartsList()` (new hook). Show schema (column list pulled from ClickHouse `system.columns`). Create button → modal form invoking `useClickhouseMartCreate()`. Drop button → `useClickhouseMartDrop()` (new) with the data-loss warning baked into the design doc.
  - `retention-panel.tsx` (~120 lines) — table of `system.tables` rows showing current TTL. Inline edit per row → `useClickhouseRetentionSet()` mutation. The existing list hook can be `useClickhouseTablesList()` (new).
  - `insights-panel.tsx` (~150 lines) — list of insights rules from the existing `starter-insights` engine via a `useInsightsRulesList()` hook. Each rule shows: name, condition (e.g. `disk_used > 90`), action (`alert.send`), enabled toggle. Create form for new rules. Per the goals-2-4-3 work, insights rules already exist server-side; this panel is the operator surface.
  - `warehouse-admin.tsx` (~80 lines) — tabbed shell composing the four panels, mirroring `<AuthzAdmin>`'s shape (tabs from `starter-ui-kit`'s `Tabs` primitive). Exports `<WarehouseAdmin>` for the route to mount.
  - `index.ts` barrel.
- **New hooks in `@nube/rubix-client-react`** — extend `hooks/clickhouse.ts` with: `useClickhouseRulesList`, `useClickhouseMartsList`, `useClickhouseMartDrop`, `useClickhouseTablesList`. Add `hooks/insights.ts` with `useInsightsRulesList`, `useInsightsRuleCreate`, `useInsightsRuleEnable`, `useInsightsRuleDisable`. Sibling `.test.tsx` per file. **Underlying REST endpoints** — if any of these REST endpoints don't exist on rubix-agent today, raise BLOCKED with a one-paragraph list of missing endpoints; the fix lands in a separate rubix-agent job, not here.
- **`rubix/frontend/src/routes/admin/warehouse.tsx`** — mounts `<WarehouseAdmin>` wrapped in `<ErrorBoundary>` like the other admin routes. i18n keys under `admin.warehouse.tabs.*`.
- **Left-nav** — add the `warehouse` entry to the admin section.
- **Smoke test** — `rubix/frontend/e2e/warehouse.spec.ts`: navigate to `/admin/warehouse`, switch through 4 tabs, assert each heading renders.

### Phase D — chrome: top header + empty states + toasts + nav

The polish layer. Without it the app feels like a typed-hooks demo.

- **Top header** — `rubix/frontend/src/components/top-header.tsx` already exists per the merged code. Augment it with:
  - Current user email + role badge, sourced from `useAuth().user` (already in the auth provider).
  - Tenant indicator if the user is in multiple tenants (read from `useTenantList()`; if 0 or 1 tenants, just show "rubix"; if 2+, show a dropdown picker — but **don't** wire tenant-switching yet, that's a separate concern requiring backend tenant-context plumbing; the dropdown just displays for now with a "switching not yet wired" tooltip).
  - A "logout" menu entry calling `useAuth().logout()`.
  - A theme toggle if `starter-ui-kit` exposes one (it does — `theme-editor` lives in starter-ui-kit per the earlier commit; just wire the toggle, don't open the full editor).
- **Toast surface for `RubixError`** — `rubix/frontend/src/components/toast-error-listener.tsx` (~80 lines): a global listener that catches uncaught `RubixError` from queries/mutations via TanStack Query's `QueryCache` `onError` callback, renders a localised toast through `starter-ui-kit`'s Toast primitive. Mount once in `main.tsx`. The existing `ErrorBoundary` keeps catching React render errors; this listener catches data-layer errors. Both coexist.
- **Empty states everywhere** — every list-rendering route gets a typed empty state component (no data → render a `<EmptyState icon=... title=... description=... action=... />` from `starter-ui-kit` if it exposes one; if not, hand-roll a minimal version in `rubix/frontend/src/components/empty-state.tsx`). Apply to: extensions list (no extensions), users list (no users), flows list (impossible since 6 bundled, but still wire defensively), warehouse tables/rules/marts/insights lists.
- **Loading skeletons** — every `useQuery` consumer renders a `<Skeleton>` (from `starter-ui-kit`) during `isLoading`. Replace any spinner-only loading state with skeleton blocks matching the eventual layout.
- **Left nav structure** — sections: `Home` (`/`), `Flows` (`/flows`), `Extensions` (`/extensions`), `Admin` (collapsible: `/admin/access`, `/admin/users`, `/admin/warehouse`), `Settings` (`/settings`). Use existing `starter-ui-kit` nav primitives.
- **Smoke test** — `rubix/frontend/e2e/chrome.spec.ts`: login, assert top-header shows the email + a logout button, assert left nav has 5 sections, click logout, assert redirect to /login.

### Phase E — closing: docs + session note + PR

- **`rubix/docs/design/frontend/README.md`** — extend with sections for: the route map (after-this-job), the rubix-flavoured `NodeKindRegistry` boot wiring, the warehouse admin surface design, the toast-error-listener pattern.
- **`rubix/docs/sessions/<today>-frontend-surfaces.md`** — closing session note: per-phase commit summary, the operator-runnable manual flow (run make start → log in → walk through every surface), test counts, list of REST endpoints that proved missing and got tracked as follow-ups.
- **`rubix/docs/scope/THIN-SLICE.md`** — update the "Goals lit up beyond the thin slice" table with the new "Frontend surfaces" row.
- **PR** — one PR off `codeless/rubix-frontend-surfaces` with phase-by-phase commits.

## Out of scope

- **No new starter packages.** Everything lives in rubix (warehouse admin) or consumes existing starter packages. `@nube/starter-ui-warehouse` is **not** created — ClickHouse admin is rubix-specific per the operator's call.
- **No modifications to existing starter UI packages.** The packages are stable; rubix-side overrides (like the custom `ai-agent` node component) register through the existing extension seams (`NodeKindRegistry::register`).
- **No flow editing.** The flow canvas is `readOnly={true}` this job. Authoring flows in the UI is a follow-up requiring substantial work on the flow-ops backend (validation, schema) that doesn't belong here.
- **No tenant switching.** The tenant indicator is display-only this job. Wiring switching needs backend session re-scoping.
- **No SDUI page rendering.** Goal 1 (dashboards SDUI) is still stubbed backend-side. The `@nube/starter-sdui-react` package stays unconsumed until Goal 1 lands.
- **No chat surface.** `@nube/starter-ui-chat` stays unconsumed in rubix this job; chat with an agent is a separate UX concern.
- **No AI flow-builder.** `@nube/starter-ui-ai-builder` stays unconsumed.
- **No blob/asset picker UI.** `@nube/starter-ui-blobs` stays unconsumed (will get used when Goal 6 reports lands and operators need to browse generated blobs).
- **No SQL code editor for CH rules.** Plain `<textarea>` is enough this job; a Monaco / CodeMirror integration is a polish follow-up.
- **No mobile-specific layout work.** Existing responsive primitives are inherited.
- **No new `starter-*` Rust crates.** If a REST endpoint needs to land for Phase C to consume, that's a follow-up rubix-agent job — flagged as BLOCKED here.
- **Live LLM in CI for e2e.** Recorded fixtures continue to back integration tests.
- **No `--no-verify`, no `--force` push.** No phasing markers in code.

## Constraints

- **R1 — One verb per file.** TS files ≤ 200 lines hard. Each panel under `components/admin/warehouse/` is one file; each route is one file.
- **R2 — Upstream-first.** No upstream changes needed for this job — confirmed by Phase 0 read of the starter packages. If during work a starter-package issue surfaces (e.g. `<AuthzAdmin>` can't be styled because a prop is missing), raise BLOCKED and file an upstream issue; don't workaround.
- **R3 — Doc-tier rule.** Code comments link `docs/design/<area>/README.md` only.
- **R4 — Errors typed.** Every hook returns `UseQueryResult<T, RubixError>`. No raw `Error`. The toast listener narrows by `RubixError.is(err)`.
- **R5 — Catalogue files.** Every new i18n key (admin.warehouse.*, flows.*, layout.nav.*) ships in both `rubix/frontend/src/i18n/{en,es}.json` same commit. R5.
- **R6 — Tests live with the code.** Each new hook gets a sibling `.test.tsx`. Each new panel gets a smoke test in `e2e/`.
- **Commit messages.** `feat(rubix-client-react):` for new hooks, `feat(rubix-frontend):` for routes and components, `test(rubix-frontend):` for e2e specs, `docs:` for the design doc and session note.

## Open questions

1. **`DecisionsPanel` data source.** `starter-ui-authz`'s `DecisionsPanel` calls something on `StarterClient`. Does that endpoint exist on rubix-agent? Phase A.1 confirms by grep; if not, panel surfaces "endpoint not wired" honest message, tracked as a follow-up.
2. **`useFlowDefinition` body source.** Does `flow-ops.list` return the full YAML body, or only metadata? Phase B.1 confirms by reading `rubix-client-ts/src/endpoints/flow_ops.ts`. If body is missing, two options: (a) add a `flow-ops.get(id)` endpoint server-side (out of scope, BLOCKED), or (b) fetch from `flows_definitions` table via an existing admin route. Default to (b) if any such route exists; (a) otherwise.
3. **Insights rules endpoints.** Does rubix-agent today expose REST for insights rules list/create/enable/disable? Per the smoke-test session note Step 5 evidence, insights ran via a hardcoded 90 threshold; the goals-2-4-3 work may have parameterised but not exposed CRUD. Phase C.2 must grep `rubix-agent/src/routes/` first; if no insights routes exist, raise BLOCKED with a one-paragraph list of needed endpoints.
4. **Code editor for SQL.** Plain `<textarea>` is fine for v1. Confirm at C.1 — if the operator demos this and finds it painful, add a Monaco/CodeMirror integration in a follow-up job.
5. **`<EmptyState>` primitive.** Does `starter-ui-kit` export one? If yes, consume. If no, hand-roll a minimal version in rubix and file an upstream issue. Phase D.1 confirms.
6. **`<Toast>` primitive.** Same question. `starter-ui-kit` likely has it; confirm at D.2.
7. **Theme toggle in the top header.** Does `starter-ui-kit` expose a `useTheme()` hook plus a toggle component? Phase D.1 confirms; if only the hook exists, hand-roll a minimal toggle button.
8. **Tenant list endpoint.** `useTenantList()` exists in `rubix-client-react`. Confirm at D.1 that it returns the current user's tenants, not all tenants (admin-only on the super-admin tenant); if it returns all tenants for a non-admin user, that's a backend AuthZ bug to file.

## References

- `packages/starter-ui-authz/` — admin panels for Phase A.
- `packages/starter-ui-flow/` — canvas + node registry for Phase B.
- `packages/starter-ui-kit/` — base primitives (tabs, toast, skeleton, empty-state, nav).
- `packages/rubix-client-react/src/hooks/` — existing hook families to extend.
- `rubix/frontend/src/routes/` — current route surface; the existing `admin/access.tsx` is the model for new admin routes.
- `rubix/frontend/src/components/top-header.tsx` — extend in Phase D.
- `rubix/docs/sessions/2026-05-24-goals-2-4-3-landed.md` — backend surface the frontend consumes.
- `rubix/SCOPE.md` R7 (flow-as-tool), R8 (extensions don't depend on rubix), the rest.
- `rubix/Makefile` — `make start` is the operator-runnable smoke flow.
- `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
