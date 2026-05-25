# 03 — Host glue (rubix implements the trait seams)

> **Tier:** scope (plan). Lifetime: weeks. Not referenced from code.
> See [README.md](./README.md).

## What this file decides

How rubix-agent wires `starter-sdui-routes` into the boot path:
which traits rubix implements, where their files live, and how
the resolver finds the entity graph at request time.

Depends on [01-storage.md](./01-storage.md) (`PgDashboardStore`)
and [02-bindings-gaps.md](./02-bindings-gaps.md) (the substrate
fixes). Unblocks [04-tools.md](./04-tools.md) and
[05-frontend-renderer.md](./05-frontend-renderer.md).

## Trait surface to implement

Four trait seams, all defined upstream. None of them is owned by
rubix — rubix only ships impls.

| Trait | Crate | What rubix's impl does |
|---|---|---|
| `EntityGraph` | `starter-ui-bindings` | Read a slot / list children on a rubix node tree entity. |
| `PageProvider` | `starter-sdui-routes` | Look up `page_ref` → `ComponentTree` from `dashboards_definitions`. |
| `QueryEngine` | `starter-sdui-routes` | Resolve the `/table` source for paginated row queries (RSQL → result rows). |
| `HandlerRegistry` + `ActionFn`s | `starter-sdui-routes` | Map `/action` calls onto rubix tools (`dashboard.update`, `flow.deploy`, …). |
| `MessageCatalogue` | `starter-ui-bindings` (added in [G6](./02-bindings-gaps.md#g6--msgkey-binding-source-for-i18n)) | Resolve `{{$msg.<key>}}` against `starter-i18n::MessageBundle` for the request locale. |

Each impl lives in **one file** in `rubix-agent/src/sdui/`:

```
rubix/crates/rubix-agent/src/sdui/
  mod.rs              ← barrel + sdui_state factory
  graph.rs            ← impl EntityGraph for RubixEntityGraph
  page_provider.rs    ← impl PageProvider for PgPageProvider
  query_engine.rs     ← impl QueryEngine for RubixQueryEngine
  handlers.rs         ← HandlerRegistry, registering each action
  catalogue.rs        ← impl MessageCatalogue (wraps starter-i18n)
  write_plan_acl.rs   ← action-time policy seam (see below)
  cache.rs            ← in-process Moka cache invalidated by NOTIFY
  notify.rs           ← NOTIFY listener (mirrors boot/flow_notify.rs)
```

## What "the rubix entity graph" actually is — v1

The graph the resolver walks is the **rubix node tree**, but in
v1 we don't have one yet. The pragmatic shape for Goal 1:

- **Entities** = principals (users), tenants, teams, flows, tools,
  rules, and "synthetic" entities exposed by the seven domain
  tools that already return structured data
  (`rubix.system.disk`, `rubix.system.flow_errors`, …).
- **`read_slot(entity_id, slot)`** dispatches by entity kind:
  - `"system"` + `slot in ("disk_percent", "disk_free_bytes", "db_lag_ms", ...)` → call the matching tool, cache for 5 s.
  - `"flow:<id>"` + `slot in ("revision_id", "last_run_at", "error_count")` → SELECT against `flows_definitions`.
  - `"user:<id>"` + `slot in ("email", "display_name", "team_id")` → SELECT against `auth_users`.
  - Anything else → `None`.
- **`read_children(entity_id)`** lists the obvious nested
  collections: a tenant's flows, a flow's recent runs, a user's
  teams.

This is intentionally a **flat synthetic graph** in v1 — it gives
the AI builder real slots to bind against without us having to
ship a generic typed-node-store. The graph trait is host-defined
by design (D5 in `DOCS/frontend/sdui/DIVERGENCE.md`); we can
replace this impl with a real node-tree later without changing
any caller.

`crates/rubix-agent/src/sdui/graph.rs` is a single file ≤ 250 LOC
with one `match` per supported entity kind. New kinds → new
match arm (verb-per-arm).

## PageProvider impl

`crates/rubix-agent/src/sdui/page_provider.rs`:

```rust
pub struct PgPageProvider {
    store: Arc<PgDashboardStore>,
    cache: Arc<DashboardCache>,
}

#[async_trait]
impl PageProvider for PgPageProvider {
    async fn lookup_page(&self, page_ref: &str) -> Option<ComponentTree> {
        if let Some(t) = self.cache.get(page_ref) { return Some(t); }
        let row = self.store.get_active(page_ref).await.ok()??;
        let tree: ComponentTree = serde_json::from_value(row.body_json).ok()?;
        // Assign synthetic ids (G4a) before caching so the subscription
        // plan and the rendered tree carry the same keys.
        let mut tree = tree;
        starter_ui_bindings::assign_synthetic_ids(&mut tree);
        self.cache.put(page_ref, &tree);
        Some(tree)
    }
}
```

Cache invalidation happens via `notify.rs`, not TTL.

## QueryEngine impl

`crates/rubix-agent/src/sdui/query_engine.rs` wraps the rubix
RSQL aggregation layer (or, until that ships, returns a
deterministic empty result set with a `Diagnostic` indicating
"feature not yet implemented for kind X"). The point in v1 is
that the trait is wired so static pages with `Component::Table`
sources render without panicking.

## HandlerRegistry — the `/action` boundary

Every dashboard action (button press, form submit) routes through
`POST /api/v1/ui/action`. The handler key (`action.handler`) maps
to a tool id, and the handler:

1. Auth-gates against `starter-authz` for the underlying tool
   permission (`rubix.dashboard:edit`, `rubix.flow:deploy`, …).
2. Calls the tool body via the shared `ToolRegistry` — same path
   as a REST `/api/v1/tools/:id` call.
3. Wraps the tool's output as the appropriate
   `ActionResponse` variant (`Toast`, `NavigateTo`, `Diagnostics`,
   `Refresh`).

```rust
let mut handlers = HandlerRegistry::new();
handlers.register("rubix.dashboard.update", action_dispatch_tool("rubix.dashboard.update"));
handlers.register("rubix.dashboard.delete", action_dispatch_tool("rubix.dashboard.delete"));
handlers.register("rubix.flow_ops.deploy",  action_dispatch_tool("rubix.flow_ops.deploy"));
// ... one register per dashboard-callable tool
```

`action_dispatch_tool(tool_id)` is a single ≤30-line factory in
`handlers.rs` — every "this action calls tool X" registration is
one line.

### `write_plan_acl.rs` — action-time policy seam

Ported from `examples/rubix-agent/crates/dashboard-transport/`
(~150 LOC). Where the tool registry says "is this principal
allowed to call this tool at all?", `write_plan_acl` answers
"is this principal allowed to **target this specific entity**
with this action right now?" — e.g. a flow operator may call
`rubix.flow_ops.deploy`, but only against flows owned by their
team.

Shape:

```rust
pub trait WritePlanAcl: Send + Sync {
    fn check(
        &self,
        principal: &Principal,
        action: &str,        // handler key
        entity: &EntityRef,  // from ActionRequest.payload
    ) -> Result<(), Diagnostic>;
}
```

`action_dispatch_tool` calls `acl.check(...)` after authz
but before invoking the tool body. On `Err(Diagnostic)`,
returns `ActionResponse::Diagnostics(vec![d])` — no panic, no
500. The v1 impl in `rubix-agent` is a stub that always returns
`Ok(())`; the seam exists so that team-scoped or row-level
policies can land later without churning every handler.

## Locale threading (Accept-Language → `$user.language`)

The resolver and action handlers both need a `locale: &str` so
`{{$msg.<key>}}` (G6) resolves against the right catalogue and
`Diagnostic` rendering for the operator picks the right language.

Cascade, evaluated by `RubixAuthLayer` for every SDUI request:

1. **`prefs.language`** on the authenticated `Principal` (set via
   the existing `starter-prefs` flow) — strongest signal,
   operator's own choice.
2. **`Accept-Language` header**, parsed by the existing
   `AcceptLanguageLayer` from `starter-i18n`. Falls back to the
   first supported locale in the q-ordered list.
3. **Server default** (`en`) when neither is present.

The chosen value lands in `request.extensions::<LocaleCtx>()` and
the `state` factory's wrapper around `sdui_router` injects it
into both `EvalContext.locale` (for `$msg`) and
`$user.language` (for any page that explicitly binds it). The
domain code never branches on the locale string — only the
`MessageCatalogue` impl does.

## Wiring it at boot

Extend `rubix-agent/src/boot/sdui.rs` (new file, ≤80 LOC):

```rust
pub fn build_sdui_router(
    pool: PgPool,
    tool_registry: Arc<ToolRegistry>,
    authz: Arc<AuthzEngine>,
) -> axum::Router {
    let store = Arc::new(PgDashboardStore::new(pool.clone()));
    let cache = Arc::new(DashboardCache::new());
    let pages = Arc::new(PgPageProvider::new(store.clone(), cache.clone()));
    let graph = Arc::new(RubixEntityGraph::new(pool, tool_registry.clone()));
    let queries = Arc::new(RubixQueryEngine::new(/* ... */));
    let handlers = build_handlers(tool_registry.clone(), authz);

    let state = SduiStateBuilder::new()
        .pages(pages)
        .graph(graph)
        .queries(queries)
        .handlers(handlers)
        .build();

    starter_sdui_routes::sdui_router(state)
}
```

`main.rs` mounts the result alongside the existing tools router:

```rust
.nest("/api/v1/ui", build_sdui_router(pool.clone(), tool_reg.clone(), authz.clone()))
```

The NOTIFY listener (`notify.rs`) is `spawn`ed from the same boot
file, holding an `Arc<DashboardCache>` for invalidation.

## Cache shape

`cache.rs` (≤80 LOC) is a `moka::sync::Cache<String, ComponentTree>`
keyed by `page_ref`, with `invalidate(page_id)` called from the
NOTIFY listener. Bounded at 256 entries — dashboards are rare
enough that this is generous.

## File layout (Rust)

```
rubix/crates/rubix-agent/src/
  sdui/
    mod.rs
    graph.rs                 ← impl EntityGraph
    page_provider.rs         ← impl PageProvider
    query_engine.rs          ← impl QueryEngine
    handlers.rs              ← HandlerRegistry + dispatch factory
    catalogue.rs             ← impl MessageCatalogue (G6)
    write_plan_acl.rs        ← action-time policy seam (~150 LOC)
    cache.rs                 ← in-process page cache
    notify.rs                ← LISTEN rubix_dashboards_definitions
  boot/
    sdui.rs                  ← build_sdui_router (≤80 LOC)
```

## Tests in the same diff

- `tests/sdui_resolve_disk_page_test.rs` — boots an in-process
  agent with PG + a bundled "disk overview" page, calls
  `/api/v1/ui/resolve`, asserts the substituted tree and the
  subscription plan.
- `tests/sdui_notify_invalidates_cache_test.rs` — write a new
  revision via the store, observe the cache invalidates and the
  next resolve returns the new body.
- `tests/sdui_action_calls_tool_test.rs` — `/action` with handler
  `"rubix.flow_ops.deploy"` reaches the tool body and returns an
  `ActionResponse::Toast`.
- `tests/sdui_authz_404_test.rs` — denied principal gets 404 from
  resolve (no body leakage).

## Acceptance

1. `make demo` boots with the SDUI router mounted; `/api/v1/ui/resolve`
   returns 200 for a seeded bundled page.
2. A live `dashboard.update` (covered in
   [04-tools.md](./04-tools.md)) flows via NOTIFY → cache invalidation
   → next resolve returns the new body.
3. The four impl files (graph / page_provider / query_engine /
   handlers) are each ≤ 250 LOC; `write_plan_acl.rs` ≤ 200 LOC;
   `catalogue.rs` ≤ 60 LOC.
4. The boot wire-up in `boot/sdui.rs` is ≤ 80 LOC.
5. A request with `Accept-Language: es` (and no `prefs.language`)
   resolves a bundled page with `{{$msg.rubix.dashboard.overview.title}}`
   to its Spanish catalogue value.
6. An action whose `WritePlanAcl::check` returns `Err(Diagnostic)`
   returns `ActionResponse::Diagnostics` and never reaches the
   tool body.
