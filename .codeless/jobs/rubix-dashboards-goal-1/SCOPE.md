# Scope — rubix-dashboards-goal-1

## Goal

Light up rubix Goal 1 (build / program dashboards via SDUI) end to end. Operator can: (a) the `dashboard-builder` skill drives the AI to author a new page through `rubix.dashboard.*` tool calls, (b) the page renders on `/dashboard/<slug>` in the rubix frontend via `<SduiPage>` from the new `@nube/starter-ui-sdui-react` package, (c) edits hot-reload through revisioned PG storage with optimistic concurrency, (d) the existing `dashboard.tsx` stub becomes a real SDUI-rendered page sourced from `dashboards_definitions`. The whole work plan is already itemised across the eight numbered files under [`rubix/docs/scope/dashboards/`](../../rubix/docs/scope/dashboards/); this codeless job folds them into five phases with the dependency order the README's graph specifies.

The infrastructure is heavily built. Verified:

- `starter-ui-ir` — component IR v5 with `page`, `grid`, `kpi`, `chart`, `table`, `form`, `tabs`, `select`, `slider`, `toggle`, `date_range`, `divider`, `custom`, `Repeat`.
- `starter-ui-bindings` — full `{{$target.slot/child}}` grammar, length-prefixed evaluator, `SubscriptionPlan` derivation, `EntityGraph` trait.
- `starter-ui-builder` — typed Rust DSL (`dashboard(...)`, `kpi`, `table`, `line_chart`, `seed_page`).
- `starter-sdui-routes` — `POST /api/v1/ui/resolve`, `/action`, `GET /table` with `PageProvider` / `QueryEngine` / `HandlerRegistry` traits, capability handshake, DoS caps.
- `starter-ui-theme` — theme tokens with migrations.
- `starter-tags` — tag substrate.
- `dashboard-builder` skill at `rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md`.
- Six tool stubs at `rubix/crates/rubix-tools/src/dashboard/` (empty bodies).

Real gaps (per the scope files, in dependency order):

1. **Storage** (01-storage.md) — `dashboards_definitions` PG table, revisions, NOTIFY, authz `ResourceSpec` registration. Mirrors PR #32's `flows_definitions` pattern.
2. **Bindings substrate** (02-bindings-gaps.md) — six polish items in `starter-ui-bindings` / `starter-ui-ir`: per-variant `Bindable` dispatch, qualifiers, `Repeat`, synthetic ids, portable-subset flag, `$msg` source. **Highest-leverage piece** — without it, only visible-text widgets template.
3. **Host glue** (03-host-glue.md) — rubix implements `EntityGraph` / `PageProvider` / `QueryEngine` / `HandlerRegistry`; wire `sdui_router` into rubix-agent boot.
4. **Tools** (04-tools.md) — fill the seven `rubix.dashboard.{create,get,update,delete,list,page_set,duplicate}` verb bodies. (Six per README + a get verb per 04 file.)
5. **Frontend renderer** (05-frontend-renderer.md) — new `@nube/starter-ui-sdui-react` package with `<SduiPage page_ref target_ref />`, subscription wiring, action dispatch. Lives in starter (R2) because the renderer is product-neutral.
6. **AI builder flow** (06-ai-builder.md) — `com.rubix.dashboard-assistant` flow with the dashboard-builder skill, the JSON dialect the LLM emits via tool args.

Out of scope per the README's non-goals: no new agent runtime, no second IR, no client-side template language, no per-tenant page partitioning v1, no widget marketplace v1, no `FetchPlan` historical pulls (07-fetch-plan.md is explicitly deferred to v2).

The success bar: a fresh operator runs `make start`, logs in, navigates to `/dashboards`, sees a list, clicks "+ New" or asks the dashboard-assistant via MCP for `"a page showing disk usage"`, gets back a real dashboard with `useDiskUsage()`-style live values rendering through `<SduiPage>`. Refresh preserves it. `make restart` preserves it. Operator edits the page via the AI flow or directly via `rubix.dashboard.update`, the page hot-reloads.

## In scope

### Phase A — substrate (parallelisable: 01 + 02)

Per the README dependency graph 01 and 02 are independent — group them as one phase with two stages.

#### A.1 — Storage: `dashboards_definitions` table + authz registration

Follow 01-storage.md verbatim. Mirrors PR #32's `flows_definitions` shape exactly.

- **`rubix-store-postgres/migrations/<NNNN>_dashboards_definitions/up.sql`** with the columns from 01-storage.md: `id ULID PK`, `tenant_id`, `page_id TEXT` (e.g. `dashboard.<slug>`), `revision_id ULID`, `body_json JSONB`, `tags TEXT[]`, `owner_actor_id`, `created_at`, `created_by`, `superseded_at NULL`. UNIQUE `(tenant_id, page_id, revision_id)`. `pg_notify('rubix_dashboards_definitions', ...)` trigger on insert/update.
- **`rubix-store-postgres/src/dashboards/store.rs`** (~200 lines verb file) exposing `PgDashboardStore` impl over a `DashboardStore` trait under `rubix-spi/src/dashboard/store.rs`. Methods: `insert_revision`, `get_active`, `list_active`, `mark_superseded`, `history`. Mirrors `FlowDefStore` shape.
- **Authz `ResourceSpec` registration** — on every successful `insert_revision`, register the page id as a `ResourceSpec` via the existing `starter-authz` engine path that the goals-2-4-3 flow definitions use. One line in `insert_revision`'s commit path; same shape as the flows-definitions registration.
- **`boot/dashboards_seed.rs`** (new) — idempotent seeder mirroring `boot/flows_seed.rs`. If any bundled dashboard YAMLs ship in `rubix-flows/dashboards/` (likely zero initially — the AI builds them), this seeds them; otherwise no-op.
- **Integration test** `rubix-agent/tests/dashboards_definitions_test.rs` — insert revision, get active, mark superseded, history. Mirrors `flow_definitions_seed_test.rs`.

Commit: `feat(rubix-store-postgres+rubix-spi): dashboards_definitions table + PgDashboardStore + authz registration`.

#### A.2 — Bindings substrate (six gaps from 02-bindings-gaps.md)

All six fit in `crates/starter-ui-bindings/` + `crates/starter-ui-ir/`. R2 strictly — these are upstream changes.

- **G1 — Per-variant `Bindable` dispatch.** Extend the existing `Bindable` trait in `starter-ui-ir/src/bindable.rs` with `visit_bindings<F>(&mut self, visit: &mut F)` and impl it for every `Component::*` variant. Refactor `starter-ui-bindings/src/substitute.rs::substitute_tree` to dispatch through `Bindable::visit_bindings` instead of the current text/heading-only match. **Highest-leverage gap — fixes charts / tables / forms / actions templating.**
- **G2 — Qualifiers in the grammar.** Per 02-bindings-gaps.md G2: extend the parser to accept `$target?` (optional, returns null if missing) and `$target!` (required, errors if missing). Currently every source is implicitly required; the qualifier makes the contract explicit.
- **G3 — `Repeat` binding context.** Per G3: when `Component::Repeat { over, .. }` walks children, push a synthetic `$item` source onto the eval cursor stack so child bindings resolve against the current iteration. Eval-side change only.
- **G4 — Synthetic ids.** Per G4: every `Repeat` child gets a synthetic id `<parent_id>.<index>` for subscription tracking. Add `Component::synthetic_id(parent_id, index)` helper in `starter-ui-ir`; consumed by `subscription.rs`.
- **G5 — Portable-subset flag.** Per G5: each component variant carries a `portable: bool` flag at compile time; the renderer's capability handshake reads it to decide whether the client can render the component. Default `true` for primitives; `false` for `Custom`. One enum addition + a const fn.
- **G6 — `$msg` source.** Per G6: add `$msg` to the source enum, resolved against an `i18n::MessageBag` passed through `EvalCtx`. This is what makes `kpi.label: "{{$msg.dashboard.kpi.disk_used}}"` work end-to-end.

Each gap is its own commit (six commits) so reviewers can read each in isolation per 02-bindings-gaps.md's "each is < ~200 LOC" note. Tests live with each commit (the existing `starter-ui-bindings/tests/` and `starter-ui-ir/tests/` directories).

Commits (in dependency order, all under `starter-ui-bindings` / `starter-ui-ir`):
1. `feat(starter-ui-ir): per-variant Bindable trait (G1)`
2. `feat(starter-ui-bindings): substitute_tree dispatches through Bindable (G1)`
3. `feat(starter-ui-bindings): qualifier grammar — $target? and $target! (G2)`
4. `feat(starter-ui-bindings): Repeat eval pushes synthetic $item source (G3)`
5. `feat(starter-ui-ir): synthetic ids for Repeat children (G4)`
6. `feat(starter-ui-ir): portable-subset flag per variant (G5)`
7. `feat(starter-ui-bindings+starter-ui-ir): $msg source with MessageBag through EvalCtx (G6)`

Seven commits across G1's two-crate split; sized to bisect cleanly.

### Phase B — host glue (03-host-glue.md)

Two stages.

#### B.1 — Implement the four traits

Per 03-host-glue.md trait surface:

- **`rubix-agent/src/sdui/entity_graph.rs`** (~150 lines) — `RubixEntityGraph` impls `starter_ui_bindings::EntityGraph` over the rubix node tree (flow runtime state). Reads slot values via the flow engine's slot-read seam.
- **`rubix-agent/src/sdui/page_provider.rs`** (~120 lines) — `PgPageProvider` impls `starter_sdui_routes::PageProvider` by looking up `page_ref` → `ComponentTree` from `PgDashboardStore::get_active`.
- **`rubix-agent/src/sdui/query_engine.rs`** (~180 lines) — `RubixQueryEngine` impls `starter_sdui_routes::QueryEngine` for the `GET /table` surface, routing queries to ClickHouse via the existing `ChClient` and to PG via the existing pool.
- **`rubix-agent/src/sdui/handler_registry.rs`** (~120 lines) — `RubixHandlerRegistry` impls `starter_sdui_routes::HandlerRegistry` for `POST /api/v1/ui/action` dispatch into the tool registry (every dashboard action is a tool call).

Each is a verb file. Tests in `rubix-agent/tests/sdui_*_test.rs`.

Commit: `feat(rubix-agent): SDUI host glue — EntityGraph + PageProvider + QueryEngine + HandlerRegistry`.

#### B.2 — Wire `sdui_router` into the boot path

- **`rubix-agent/src/boot/sdui.rs`** (~120 lines verb file) — `build_sdui_router(cfg, pg_pool, ch_client, tool_registry) -> Router` constructs the four trait impls and calls `starter_sdui_routes::router(...)`. Returns a router mounted under `/api/v1/ui/*`.
- **`rubix-agent/src/main.rs`** — merge the new router. Same pattern as the extensions and flow_events routers. **One known merge point with the in-flight `rubix-flow-live-tick-demo` job** — both jobs touch `main.rs` to add a new router. Adjacent additions; second-to-merge rebases trivially.

Commit: `feat(rubix-agent): mount SDUI router under /api/v1/ui`.

### Phase C — tools (04-tools.md)

Fill the seven verb bodies. Five stages.

The seven verbs per 04-tools.md:

| Verb | Body |
|---|---|
| `rubix.dashboard.create` | Create a new page from a body JSON or duplicate-as-starting-point of a bundled template. Reversible: `delete` reverses. |
| `rubix.dashboard.get` | Fetch the active body of a `page_ref`. |
| `rubix.dashboard.update` | Insert a new revision with optimistic concurrency via `expected_revision_id`. Reversible: `undo.last` re-supersedes back to prior. |
| `rubix.dashboard.delete` | Supersede every revision of a `page_id`. Refused for `created_by="system"`. Reversible: `undo.last` re-inserts the previously active revision. |
| `rubix.dashboard.list` | Active pages in the caller's tenant, filtered by tags / owner / search. |
| `rubix.dashboard.duplicate` | Snapshot an existing page into a new `page_id`. Reversible: `delete` of the duplicate. |
| `rubix.dashboard.page_set` | Update one slot on a live page (e.g. selected tab). Used by the action surface to mutate page state via `update_target_slot` chokepoint. |

#### C.1 — Reads: `get` + `list`

Both read-only, no `Reversible`. Smallest stage; lands the DTO + descriptor pattern that the others mirror.

- **DTOs in `rubix-spi/src/dto/dashboard/{get,list}.rs`** with utoipa `ToSchema`.
- **Verb bodies in `rubix-tools/src/dashboard/{get,list}.rs`** dispatching through `PgDashboardStore`.
- **MessageKeys** `rubix.dashboard.fetched`, `rubix.dashboard.listed`, `rubix.dashboard.get.not_found` — en + es catalogues same commit.
- **Sibling test** asserting the round-trip via the tool dispatch path.

Commit: `feat(rubix-tools+rubix-spi): dashboard.get + dashboard.list`.

#### C.2 — Writes: `create` + `update`

Both reversible via the existing `starter-undo` dispatch (per goals-2-4-3 pattern).

- **DTOs** in `rubix-spi/src/dto/dashboard/{create,update}.rs`.
- **`create.rs`** writes a new `(page_id, revision_id)` row via `PgDashboardStore::insert_revision`. Registers a `ResourceSpec` via authz. Returns `{ summary, page_id, revision_id, created_at_ms }`.
- **`update.rs`** writes a new revision with optimistic concurrency — input `expected_revision_id`; if it doesn't match the current active row, return `rubix.dashboard.update.conflict` Diagnostic. Marks the prior revision superseded in the same transaction.
- **MessageKeys** `rubix.dashboard.created`, `rubix.dashboard.updated`, `rubix.dashboard.update.conflict`, `rubix.dashboard.create.duplicate_id` — en + es same commit.
- **Sibling tests** covering create → undo (delete) → restore, update → conflict-on-stale-revision.

Commit: `feat(rubix-tools+rubix-spi): dashboard.create + dashboard.update`.

#### C.3 — Writes: `delete` + `duplicate`

- **`delete.rs`** supersedes every revision of `page_id`. Refused with `rubix.dashboard.delete.refused_system` if `created_by="system"` (bundled pages — system protection). Reversible: undo re-inserts the previously active revision.
- **`duplicate.rs`** reads the active revision of the source `page_id`, writes a new `(new_page_id, fresh_revision_id)` row with the same body. Reversible: delete of the duplicate.
- **MessageKeys** `rubix.dashboard.deleted`, `rubix.dashboard.delete.refused_system`, `rubix.dashboard.duplicated`, `rubix.dashboard.duplicate.source_not_found` — en + es same commit.

Commit: `feat(rubix-tools+rubix-spi): dashboard.delete + dashboard.duplicate`.

#### C.4 — `page_set` (action-chokepoint write)

- **`page_set.rs`** mutates one slot on a live page (target an entity, set a slot value). Used by the action surface (`HandlerRegistry` impl from Phase B). Returns the new slot value for echo.
- This is NOT a revision write — it's a runtime slot write into the page's entity tree (which the `EntityGraph` impl reads). Routes through the same write chokepoint flows use (per R2 — one slot-write chokepoint).
- **MessageKey** `rubix.dashboard.page_set.applied` — en + es same commit.

Commit: `feat(rubix-tools+rubix-spi): dashboard.page_set runtime slot write`.

#### C.5 — Integration test + dispatch wiring

- **`rubix-tools/src/dashboard/mod.rs`** updated to export the seven verbs into the tool registry.
- **`rubix-agent/src/boot/mcp/register.rs`** (or wherever the tool registry assembly lives) — register the seven tools so they auto-surface as MCP tools per R7.
- **`rubix-agent/tests/dashboard_crud_test.rs`** end-to-end: create → get → update → conflict-on-stale → undo → delete → duplicate → list-with-filter → page_set. Uses testcontainers PG. Mirrors the goals-2-4-3 integration test shape.

Commit: `test(rubix-agent): dashboard CRUD + page_set end-to-end`.

### Phase D — frontend renderer + AI builder (05 + 06)

Two stages — 05 (the new starter package) lands first, 06 (the rubix-side flow + adoption) builds on it.

#### D.1 — `@nube/starter-ui-sdui-react` package (05-frontend-renderer.md)

R2 strictly — lives in starter because the IR is starter's, the routes are starter's, and any other starter consumer (notes example, future SaaS apps) reuses the renderer. Zero I/O per R6 (HOW-TO-CODE.md); the caller supplies the transport adapter.

- **`packages/starter-ui-sdui-react/`** (new package mirroring `starter-ui-flow`'s shape):
  - `package.json` — name `@nube/starter-ui-sdui-react`, peer-deps React 19 + `@tanstack/react-query` 5 + `@nube/starter-ui-ir`, depends on `@nube/starter-client-ts` workspace.
  - `tsconfig.json` extending starter-client-ts's; `vitest.config.ts`.
  - `src/index.ts` barrel.
  - `src/sdui-page.tsx` (~250 lines, may split) — `<SduiPage page_ref target_ref />` consumes `useSduiResolve(page_ref, target_ref)` + `useSduiSubscriptions(plan)` + dispatches actions via `useSduiAction()`. All three hooks expect a `SduiTransport` provided via context.
  - `src/transport/index.ts` — `SduiTransport` interface (`resolve`, `subscribe`, `action`). `HttpSduiTransport` consuming a `StarterClient` is the concrete impl. Adapter shape mirrors `starter-ui-ai-builder`'s `http.ts` adapter.
  - `src/renderer/index.ts` — one verb file per IR variant (`render-page.tsx`, `render-grid.tsx`, `render-kpi.tsx`, `render-chart.tsx`, `render-table.tsx`, `render-form.tsx`, `render-tabs.tsx`, `render-select.tsx`, `render-slider.tsx`, `render-toggle.tsx`, `render-date-range.tsx`, `render-divider.tsx`, `render-custom.tsx`, `render-repeat.tsx`). Each ≤ 150 lines TS. Built-in widgets use `@nube/starter-ui-kit` primitives.
  - `src/provider/sdui-provider.tsx` — `<SduiProvider transport={...}>` context.
  - Sibling `.test.tsx` per render verb file using msw or fetch-mock.
- **Add to `pnpm-workspace.yaml`.**
- **`packages/starter-ui-sdui-react/README.md`** — present-tense, the mount pattern, the transport seam, the renderer-per-variant convention.

`pnpm --filter @nube/starter-ui-sdui-react typecheck + test` green.

Commit: `feat(starter-ui-sdui-react): new package — SduiPage + per-variant renderers + transport seam`.

#### D.2 — Frontend adoption + dashboard-assistant flow + bundled-dashboard demo (06-ai-builder.md)

Three concrete pieces in one stage:

- **`rubix-flows/flows/dashboard-assistant.yaml`** — rewrite from the goal-1 stub (currently returns `rubix.goal.not_wired`) into a real flow per 06-ai-builder.md. Root `ai-agent` with `skill_hint: com.rubix.dashboard-builder`, `allowed_tools: [rubix.dashboard.create, rubix.dashboard.update, rubix.dashboard.get, rubix.dashboard.list, rubix.dashboard.duplicate, rubix.dashboard.delete, rubix.dashboard.page_set, rubix.undo.last]`.
- **Delete the goal-1 stub** at `rubix-tools/src/dashboard/assistant.rs` (the stub returning `rubix.goal.not_wired`); the primary tool from the flow is now real.
- **`rubix-flows/dashboards/disk-overview.json`** (new bundled page, ~80 lines) — a worked-example dashboard with one `kpi` widget bound to `useDiskUsage`-style live data plus a `chart` showing the disk-history. Seeded by `boot/dashboards_seed.rs` (created in A.1) into `dashboards_definitions` with `created_by="system"`.
- **`rubix-client-ts/src/endpoints/dashboard.ts`** — new endpoint family covering the seven verbs, mirroring `flow_ops.ts`.
- **`rubix-client-react/src/hooks/dashboard.ts`** — new hook family: `useDashboardList`, `useDashboardGet`, `useDashboardCreate`, `useDashboardUpdate`, `useDashboardDelete`, `useDashboardDuplicate`, `useDashboardPageSet`. Query keys `['rubix','dashboard',...]`.
- **`rubix-frontend/src/routes/dashboards/index.tsx`** — new list route. `useDashboardList()`, table with create button.
- **`rubix-frontend/src/routes/dashboards/$pageId.tsx`** — new detail route mounting `<SduiPage page_ref={pageId}>` from the new package; transport supplied via `<SduiProvider transport={httpTransport(rubixClient)}>`.
- **Replace `rubix-frontend/src/routes/dashboard.tsx`** (the current hand-coded stub with `SPARK_DEVICES` mocks) with a redirect to `/dashboards/disk-overview` (the bundled page from above) — or delete the route entirely if `/dashboards/...` covers the surface. Keep behaviour change minimal — operator visits `/dashboard` and lands on the real page.
- **Provider wiring** — mount `<SduiProvider>` once in `main.tsx` alongside the existing `<RubixClientProvider>`.
- **Playwright spec** `rubix-frontend/e2e/dashboards.spec.ts`: log in, navigate to `/dashboards/disk-overview`, assert the kpi renders a numeric value (live from `useDiskUsage`-equivalent), assert the chart renders, click the chart's time-range toggle, assert the `page_set` action lands.
- **Two MessageKey families** added to `rubix/frontend/src/i18n/{en,es}.json`: `dashboards.*` for the list route, `dashboard.kpi.*` and `dashboard.chart.*` for the bundled page's IR labels.

Two commits split for review surface:
1. `feat(rubix-client-ts+rubix-client-react): dashboard endpoint family + hooks`.
2. `feat(rubix-frontend+rubix-flows): dashboard-assistant flow + /dashboards routes + bundled disk-overview page + Playwright e2e`.

### Phase E — closing docs + session note + PR

- **Promote the scope files to design docs.** Per the README's "When the file's work ships, promote the present-tense parts into `docs/design/sdui/<area>/README.md` and delete the scope file (or leave a one-line back-reference)." Six scope files (01–06) get promoted; 07 stays (deferred); 08 gets emptied as questions resolve.
- **`rubix/docs/design/sdui/`** — new directory with `README.md` + sub-docs: `storage.md` (from 01), `bindings.md` (from 02), `host-glue.md` (from 03), `tools.md` (from 04), `renderer.md` (from 05), `ai-builder.md` (from 06). Each present-tense; each citable from code comments per R3.
- **`rubix/docs/sessions/<today>-dashboards-goal-1-landed.md`** — closing session note: per-phase commits, operator-runnable manual flow (boot → log in → navigate to /dashboards → see list → click disk-overview → see live kpi + chart → ask the dashboard-assistant via MCP to create a new page → see it appear in the list → edit via update → refresh → preserved), test counts.
- **`rubix/docs/scope/THIN-SLICE.md`** — flip Goal 1 from "stubbed" to "real" with the evidence link.
- **`rubix/docs/scope/dashboards/`** — replace each promoted scope file with a one-line `Promoted to docs/design/sdui/<area>/README.md` redirect, or delete if the design doc supersedes cleanly.
- **PR** — one PR off `codeless/rubix-dashboards-goal-1` with phase-by-phase commits.

Commit: `chore(docs): promote dashboards scope to docs/design/sdui + close out Goal 1 + open PR`.

## Out of scope

- **`FetchPlan` historical pulls (07-fetch-plan.md).** Deferred to v2; the scope file stays in `docs/scope/dashboards/` as the v2 hand-off.
- **Per-tenant page partitioning.** Pages carry `tenant_id`; list filters on the principal's tenant; full schema-per-tenant is a later migration.
- **Widget marketplace / extension-contributed renderers in v1.** `Component::Custom` is supported by the IR + capability handshake; the AI builder limits itself to the built-in catalogue.
- **Client-side template language.** Templates live and die on the server (per the README non-goal).
- **Second IR / second binding grammar.** Everything renders via `starter-ui-ir` (per the README non-goal).
- **Themed customisation per dashboard.** The existing `starter-ui-theme` provides app-wide tokens; per-page theming is a v2 polish.
- **WebSocket transport for subscriptions.** SSE via the flow-events route is enough for v1 live values; WS is a future job.
- **Authoring a fancy WYSIWYG dashboard editor.** v1 authoring is via the AI builder (`dashboard-assistant` flow). A WYSIWYG editor is a separate v2 job.
- **Skeleton / loading states beyond what `starter-ui-kit` provides.** Reuse existing `<Skeleton>` and `<Empty>` primitives.
- **Live LLM in CI.** Recorded fixtures back the AI-builder e2e per the existing pattern.
- **No `--no-verify`, no `--force`.** No phasing markers in code.

## Constraints

- **R1 — one verb per file.** Rust ≤ 400 lines hard. TS ≤ 200 lines hard. Each verb (`get`, `list`, `create`, `update`, `delete`, `duplicate`, `page_set`) is its own file; each renderer (kpi, chart, table, ...) is its own file.
- **R2 — upstream-first.** A.2's six bindings gaps land in `starter-ui-bindings` / `starter-ui-ir` first. D.1's `@nube/starter-ui-sdui-react` lives in `packages/` not `rubix/packages/` because every starter consumer can use it.
- **R3 — doc-tier rule.** Code comments link `docs/design/sdui/<area>/README.md` only (the design docs landed in Phase E). Never the scope files. `./rubix/scripts/lint-doc-refs.sh` enforces it.
- **R4 — tool outputs are `Diagnostic` + structured data.** SDUI `/resolve` returns a `ComponentTree`; tool responses return `Diagnostic` + data per existing convention.
- **R5 — catalogue files are the source of truth for MessageKeys.** Every new key in both `rubix-spi/catalogues/en.json` and `es.json` same commit.
- **R6 — tests live with the code in the same commit.**
- **R7 — every dashboard tool auto-surfaces as an MCP tool via the registry.** No bespoke MCP wiring.
- **R8 — extensions don't depend on rubix.** This job doesn't touch extensions.
- **R10 — reverse-DNS ids.** `rubix.dashboard.*` for verbs; `dashboard.<slug>` for page ids; `com.rubix.dashboard-assistant` for the flow.
- **R12 — MCP resource URIs distinct from SDUI page ids.** `rubix://dashboard/pages` is the MCP resource URI; `dashboard.<slug>` is the page id. Both honoured.
- **Commit messages.** `feat(starter-ui-bindings):`, `feat(starter-ui-ir):`, `feat(starter-ui-sdui-react):` for upstream; `feat(rubix-store-postgres):`, `feat(rubix-spi+rubix-tools):`, `feat(rubix-agent):`, `feat(rubix-client-ts+rubix-client-react):`, `feat(rubix-frontend+rubix-flows):` for downstream; `chore(docs):` for the closing.

## Open questions

Most have defaults documented in 08-open-questions.md; the codeless agent should fold the answers it picks into the relevant numbered scope file as Phase E.1 promotion happens.

1. **Page storage shape** (Q1 in 08-open-questions.md) — `rubix-owned PG table` per 01-storage.md. Decided.
2. **`page_id` shape** — `dashboard.<slug>` reverse-DNS-ish. Decided per 04-tools.md.
3. **Action surface** — every action is a tool call dispatched through the existing tool registry. Decided per 03-host-glue.md.
4. **Bundled-dashboards directory** — `rubix/crates/rubix-flows/dashboards/*.json`. New directory; seeded by `boot/dashboards_seed.rs`. Decided.
5. **`page_set` reversibility** — page_set is a runtime slot write (not a revision); NOT reversible via `undo.last`. Operator can revert by setting the slot back. Document the choice in the design doc + 04-tools.md promotion.
6. **AI dialect for `dashboard-assistant`** — per 06-ai-builder.md, the LLM emits typed tool args; the design doc captures the precise shape. The codeless agent reads 06-ai-builder.md verbatim for the dialect spec.
7. **`@nube/starter-ui-sdui-react` peer-dep on `@nube/starter-ui-ir`** — confirm the IR is exposed as a TS package today. If only Rust, this becomes a small upstream addition (a generated `.d.ts` from the Rust types, or a hand-written TS mirror). Phase D.1 confirms.

## References

- `rubix/docs/scope/dashboards/README.md` — the master index.
- `rubix/docs/scope/dashboards/01-storage.md` — Phase A.1.
- `rubix/docs/scope/dashboards/02-bindings-gaps.md` — Phase A.2.
- `rubix/docs/scope/dashboards/03-host-glue.md` — Phase B.
- `rubix/docs/scope/dashboards/04-tools.md` — Phase C.
- `rubix/docs/scope/dashboards/05-frontend-renderer.md` — Phase D.1.
- `rubix/docs/scope/dashboards/06-ai-builder.md` — Phase D.2.
- `rubix/docs/scope/dashboards/07-fetch-plan.md` — out of scope (v2).
- `rubix/docs/scope/dashboards/08-open-questions.md` — answered as the work lands.
- `crates/starter-ui-ir/`, `crates/starter-ui-bindings/`, `crates/starter-ui-builder/`, `crates/starter-sdui-routes/`, `crates/starter-ui-theme/`, `crates/starter-tags/` — substrate.
- `rubix/crates/rubix-tools/src/dashboard/` — stub files to fill.
- `rubix/crates/rubix-skills/skills/dashboard-builder/SKILL.md` — the AI's skill file.
- `rubix/crates/rubix-flows/flows/dashboard-assistant.yaml` — the goal-1 flow stub to rewrite.
- `rubix/frontend/src/routes/dashboard.tsx` — the hand-coded stub to replace.
- `rubix/docs/sessions/2026-05-25-handover-flow-crud-and-orientation.md` — current handover + codeless runbook.
- `rubix/SCOPE.md`, `rubix/HOW-TO-CODE.md`, `rubix/FILE-LAYOUT.md`, `rubix/NEW-SESSION.md`.
