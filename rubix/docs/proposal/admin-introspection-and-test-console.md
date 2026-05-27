# Proposal: Admin introspection APIs + test console

Status: Draft (v2, peer-review applied 2026-05-28)
Owner: ap@nube-io.com
Date: 2026-05-28
Related: [extensions-north-star/PROGRESS.md](../scope/extensions-north-star/PROGRESS.md), [extensions-north-star/README.md](../scope/extensions-north-star/README.md), [flow-storage-and-undo-redo.md](./flow-storage-and-undo-redo.md), [warehouse-explorer-visual-rebuild.md](../scope/warehouse-explorer-visual-rebuild.md)

## Changelog from v1

Applied peer-review feedback:
- **§Security**: new top-level section. Tenant context on invoke is now a required field, not "admin-tenant". Separate `admin:read` and `admin:invoke` roles. DB role for previews is explicitly the tenant's RLS-scoped role, not superuser. Audit logging moved from missing to in-scope.
- **§Route catalog**: wrapped `RouteRegistrar` (single change site) instead of per-mount-site bookkeeping.
- **§OpenAPI**: pulled forward from "defer" to **day-one** output of the catalog. No second migration later.
- **§Schemas**: schema-not-declared becomes a **CI gate**, not a runtime fallback. M4 (was "ongoing backfill") collapses into M1 as a forcing-function sweep.
- **§Slots**: moved from M1 to M3. `HostSlotRegistry` is new infra, not wiring; admitting that.
- **§Envelope**: dropped redundant `kind` outer field; `source` is now a tagged union, not a string prefix.
- **§API shape**: primary endpoint is `GET /admin/registry?kinds=…` (MCP-style); per-kind URLs become documented sugar.
- **§MCP alignment**: tool list/call envelope deliberately matches MCP `tools/list` / `tools/call` so `starter-mcp` and the console share one surface.
- **§Frontend**: confirmed React stack; `@nube/starter-ui-warehouse-explorer` already exists and slots in as the template/table preview surface — we do not rebuild it.
- **§Pagination**: every list endpoint gets `?limit=&cursor=` from M1.

## Why

The extension architecture has shipped a lot of surface — Phases 1, 2, and B are at ✅ — but **an operator landing on this server has no way to see what it can do.** Only `GET /api/v1/extensions` (manifest dump) and `POST /api/v1/tools/{id}` (dispatch, no listing) are exposed.

Three moments where this becomes real pain:
1. **First-time setup** — "what's installed and what does it do?" has no answer.
2. **Extension authoring** — authors can't see what slots / contracts the host actually accepts.
3. **Debugging** — when a tool / rule / template fails, there's no canonical place to view its declared schema vs the payload that was sent.

## Current state — what's already exposed

| Item | In-process registry | List API | Schema API | Verdict |
|---|---|---|---|---|
| Extensions | ✓ `ExtensionRegistry` | ✓ `GET /api/v1/extensions` | ✓ (full manifest) | **OK** |
| Tools | ✓ `ToolRegistry` ([rubix-agent/src/registry.rs](../../crates/rubix-agent/src/registry.rs)) | ✗ | ✗ (only `POST` dispatch) | wiring |
| Skills | ✓ `SkillRegistry` ([starter-skills/src/registry.rs:333](../../../crates/starter-skills/src/registry.rs)) | ✗ | ✗ | wiring |
| Nodes (flow kinds) | ✓ `StaticNodeKindRegistry` ([starter-flow-nodes/src/node_registry.rs](../../../crates/starter-flow-nodes/src/node_registry.rs)) | ✗ in rubix-agent | ✗ in rubix-agent | wiring |
| Anomaly rules | ✓ `RuleRegistry` ([starter-insights/src/registry.rs:63](../../../crates/starter-insights/src/registry.rs)) | ✗ | ✗ | wiring |
| Warehouse tables | manifest-only, no central catalog | ✗ | ✗ | wiring + small projection |
| Warehouse templates | ✓ `TemplateRegistry` ([starter-ext-host/src/warehouse.rs:65](../../../starter-extensions/crates/starter-ext-host/src/warehouse.rs)) | ✗ | ✗ | wiring |
| REST routes (contributed) | mounted ad-hoc via `CompositeRestDispatcher` | ✗ | ✗ | **new infra** (registrar) |
| UI slot **catalog** (what host accepts) | ✗ does not exist | ✗ | ✗ | **new infra** |
| UI slot **fills** (what extensions provide) | derivable from manifests | ✗ (only per-ext route) | ✗ | wiring |
| Warehouse data preview | `@nube/starter-ui-warehouse-explorer` (forked sql-studio) — **already shipped** | n/a (host-side) | n/a | reuse, don't rebuild |
| Chat-driven tool dispatch | `/chat` route + `POST /api/v1/chat/stream` SSE ([rubix/frontend/src/routes/chat.tsx](../../frontend/src/routes/chat.tsx), [rubix/crates/rubix-agent/src/routes/chat_stream.rs](../../crates/rubix-agent/src/routes/chat_stream.rs)) — **already shipped**, renders inline `tool_use` pills as the model dispatches | (host-side stream) | n/a | reuse as third invocation path |

## Goal

Two deliverables:

1. **Admin introspection APIs** — one consistent shape across every registry, MCP-compatible, secured by an explicit tenant + role model, audit-logged.
2. **An admin/test console page** that consumes those APIs and lets an operator browse every registry, view JSON Schema for any item, fill in a form, fire a call, see the response. **Warehouse table / template preview is delegated to the existing `starter-ui-warehouse-explorer` package** — not reimplemented.

## Non-goals

- A new transport. Everything rides existing Axum REST.
- A new auth model — reuse the existing principal/tenant. But: define the role split (§Security) explicitly.
- Rebuilding warehouse data preview UI. `starter-ui-warehouse-explorer` exists and mounts at `/admin/warehouse-explorer`.
- Replacing `/api/v1/extensions`. It stays as the canonical "what's installed" view; new APIs are cross-cutting.

## Security model (was a red flag in v1; now load-bearing)

The single most dangerous endpoint family is `/admin/*/invoke` and `/admin/templates/*/query`. v1 hand-waved "admin-tenant context." That was wrong. Spelled out now:

### Roles

| Role | Grants |
|---|---|
| `admin:read` | All `GET /api/v1/admin/*` — catalog browsing only. No invoke, no query. |
| `admin:invoke` | `POST /api/v1/admin/tools/:id/invoke`, `POST /api/v1/admin/templates/:name/query`, table preview. Implies `admin:read`. |

A user can hold `admin:read` without `admin:invoke`. The console's browse UI is usable by a junior operator; the run-buttons are gated.

### Invoke / query semantics

- **`tenant` is a required field** in every `POST /admin/tools/:id/invoke` and `POST /admin/templates/:name/query` request body. There is no "admin tenant" and no default. Server returns 400 if absent.
- The server **constructs a `Principal` for the target tenant** and dispatches the tool with it. The admin's own principal is recorded as `actor`, not used as the tool's tenant scope.
- Cross-tenant invoke is explicit by design: an `admin:invoke` user is choosing to act as tenant X, and that choice is logged.

### DB role for warehouse preview

- Table preview (`SELECT … LIMIT n`) and template query run as the **same RLS-scoped DB role the tenant uses** at runtime. Not as the agent's superuser/migration role.
- This means: if RLS or per-tenant row filters are configured in the warehouse, the admin console inherits them. The "admin" prefix is **not** an RLS bypass.
- If this is impossible today (single DB role, no RLS), then table preview is **deferred** until that's fixed. Better to ship a working catalog (M1) than a leaky preview.

### Audit logging (in-scope, not deferred)

Every `/api/v1/admin/*` call — including `GET` reads — is logged to `starter-audit`. Fields:

```
ts, actor (principal), role_used, route, method,
target_tenant (for invoke/query, else null),
request_body_hash, response_status, response_body_hash,
latency_ms
```

Body hashes (not bodies) so we can detect "this admin ran the same query 50 times" without storing PII. Bodies-on-demand behind a separate elevated permission, if ever needed.

### Idempotency / rate limits

- `POST /admin/templates/:name/query` and table previews are not idempotent (they hit the warehouse). Apply per-actor rate limit (initial: 30/min per actor).
- `POST /admin/tools/:id/invoke` inherits the tool's own idempotency (most aren't); no additional layer, but the audit log gives forensic visibility.

## API design

### Uniform envelope (MCP-aligned)

Every item across every registry returns:

```json
{
  "items": [
    {
      "id": "rubix.warehouse.ingest",
      "label": "Warehouse ingest",
      "summary": "...",
      "source": { "kind": "builtin" }
        // or { "kind": "extension", "id": "com.rubix.example" }
        // or { "kind": "starter" }
      ,
      "input_schema":  { ... },
      "output_schema": { ... },
      "metadata": { /* per-kind extras */ }
    }
  ],
  "next_cursor": "abc..."   // null when no more pages
}
```

The shape of `id`/`label`/`summary`/`input_schema` deliberately matches MCP's `tools/list` payload so `starter-mcp` and the console share one projection. The tool-list response is byte-identical between `/api/v1/admin/registry?kinds=tool` and the MCP `tools/list` handler.

### Endpoints

Primary:

```
GET /api/v1/admin/registry?kinds=tool,node,rule,template,table,skill,route,slot
                          &source=builtin|extension:<id>|starter
                          &limit=50&cursor=…
```

Returns a single envelope keyed by kind:

```json
{
  "tool":     { "items": [...], "next_cursor": null },
  "node":     { "items": [...], "next_cursor": "..." },
  ...
}
```

Per-kind URL sugar (same envelope, single kind):

```
GET /api/v1/admin/tools                  GET /api/v1/admin/tools/:id
GET /api/v1/admin/nodes                  GET /api/v1/admin/nodes/:kind
GET /api/v1/admin/rules                  GET /api/v1/admin/rules/:id
GET /api/v1/admin/templates              GET /api/v1/admin/templates/:name
GET /api/v1/admin/tables                 GET /api/v1/admin/tables/:name
GET /api/v1/admin/skills                 GET /api/v1/admin/skills/:id
GET /api/v1/admin/routes                 GET /api/v1/admin/slots
GET /api/v1/admin/overview               (counts only; cheap)
GET /api/v1/admin/openapi.json           (projection of the catalog; see below)
```

Invocation (gated `admin:invoke`):

```
POST /api/v1/admin/tools/:id/invoke
  body: { tenant: "<uuid>", input: { ... } }

POST /api/v1/admin/templates/:name/query
  body: { tenant: "<uuid>", params: { ... } }
```

Reduces ~16 handlers to ~3 (the registry projection, plus the two invoke handlers). Per-kind URLs route to the projection with a fixed kind filter.

### Pagination

Every list response carries `next_cursor`. Cursors are opaque strings (impl: base64 of (last_id, page_size)). Default page size 50, max 200.

## Wrapped route registrar (replaces v1's "5 lines per mount site")

```rust
// In rubix-agent/src/routes/registrar.rs
pub struct RouteRegistrar {
    router: Router,
    catalog: Vec<RouteEntry>,
}

impl RouteRegistrar {
    pub fn route<H>(mut self, method: Method, path: &str, handler: H, meta: RouteMeta) -> Self
    where /* axum handler bounds */
    { /* records meta into catalog AND calls Router::route */ self }

    pub fn build(self) -> (Router, RouteCatalog) { ... }
}
```

Single chokepoint. A route that doesn't go through `RouteRegistrar::route` won't compile against the boot composer (which takes `RouteRegistrar`, not `Router`, until `build()` is called).

This is also the **OpenAPI source**: `RouteEntry` carries `request_schema`, `response_schema`, `description`. `GET /api/v1/admin/openapi.json` is a pure projection. No second migration when someone asks for OpenAPI later.

## Schemas — CI gate, not runtime fallback

Sweep at M1 boundary: add a workspace test that fails if any item registered with a registry lacks an `input_schema` (where applicable). Targets:

- Every `Tool::definition()` must declare `input_schema` and `output_schema` (or explicit `None` for parameterless tools — the envelope distinguishes "no input" from "schema not declared")
- Every `NodeBehavior::config_schema()` must be non-empty (already enforced by schemars derive)
- Every `Rule` must declare its wire schema
- Every `TemplateSpec.params_schema` must validate as JSON Schema
- Every `ContributeWarehouseTable.columns[*]` must declare a type and (for non-nullable) a description

Backfill is finite, not "ongoing." First M1 PR fails the build until the sweep is green.

Console envelope distinguishes:

| `input_schema` value | Meaning | UI behavior |
|---|---|---|
| `{}` or `{ "type": "object", "properties": {} }` | Tool takes no input | "Invoke" button, no form |
| present, non-empty | Normal | Render rjsf form |
| `null` / absent | Not declared (CI gate failed) | Should never happen post-M1; show error |

## UI slots — admit it's new infra

v1 conflated "extensions declare `contributes.ui`" with "host knows what slots exist." They're different. Build:

```rust
// In rubix-agent/src/slots/registry.rs
pub struct HostSlotRegistry {
    slots: BTreeMap<SlotId, SlotSpec>,
}

pub struct SlotSpec {
    pub id: SlotId,
    pub label: String,
    pub mount_path: String,        // e.g. "dashboard:left-panel"
    pub props_schema: RootSchema,  // what the host passes into the slot
    pub max_fills: Option<u32>,    // None = many; Some(1) = singleton
}
```

The registry is populated at boot from a `slots.yaml` (or compile-time const). `/api/v1/admin/slots` returns the union of:
- **Declared** slots (from `HostSlotRegistry`) — what extension authors can target
- **Filled** slots (derived from each extension's `contributes.ui`) — what's currently mounted

This is the actual answer to "what UI plug-ins can I write?" — v1's "list slots" only showed fills, which is the less useful half.

Moves from M1 to M3.

## Frontend

**Stack confirmed:** React (vite, `rubix/frontend/`). `@rjsf/core` is appropriate. ~200KB is acceptable for an admin-gated page.

### Tool dispatch is one component, surfaced in four places

The platform already has several places where a tool is dispatched and its response is rendered. Today they are (or would be) separate implementations of the same idea:

| Surface | What dispatches | Where it renders | State today |
|---|---|---|---|
| Direct API | `POST /api/v1/tools/:id` | Programmatic — no UI | shipped |
| Chat | LLM picks tool, fires via `chat_stream.rs` | Inline `⚙ name` pills in `/chat` ([chat.tsx](../../frontend/src/routes/chat.tsx)) | shipped |
| MCP | `tools/call` via `starter-mcp` | External clients (Claude Desktop, etc.) | shipped |
| Admin tool-tester (this proposal) | Operator clicks Invoke | Form + streaming response in `/admin/tools/:id` | M2 |
| Flow editor "test this node" | Operator triggers single-node dry-run | In-canvas tool result panel | future |

Every one of those is the same problem: **render a tool's input schema as a form, dispatch it, show the streaming response (text, tool-use pills, errors, completion)**. We must not build that five times.

#### The shared component: `<ToolTester />`

A single React component, shipped as a package, that takes a tool id (or full tool definition) and renders the whole dispatch loop. Used in:

- `/admin/tools/:id` — the canonical, full-size view
- `/admin/templates/:name` — same component, parameter form pre-bound to the template's `params_schema`
- `/chat` — when the LLM emits `tool_use`, the inline pill expands into the same `<ToolTester />` (read-only view of input + streaming output, no re-fire button)
- Flow editor node panel — "test this node with these inputs" uses `<ToolTester toolId={node.kind} ... />`
- Any extension UI that wants a "try this tool" button

```
packages/starter-ui-tool-tester/
  src/
    ToolTester.tsx              // the full widget: form + dispatch + response
    InputForm.tsx               // JSON Schema → form (uses starter-ui-schema-form)
    ResponsePanel.tsx           // streaming response (SSE frame decode + render)
    useToolDispatch.ts          // headless hook for non-rendering consumers
    frames.ts                   // ChatFrame / ToolFrame types — single source of truth
    index.ts
```

#### Server-side: one frame shape, not four

The current `chat_stream.rs` produces `ChatFrame { connected | text | tool_use | done | error }`. The new `POST /api/v1/admin/tools/:id/invoke` MUST produce the same frames (minus `connected`/`tool_use` which are chat-loop concepts; plus a `progress` frame if tools want it).

Concretely:
- Extract the frame type from `chat_stream.rs` into a shared module (`rubix-agent/src/routes/stream_frames.rs` or move to `starter-spi`)
- `/admin/tools/:id/invoke` reuses it
- MCP `tools/call` ideally maps onto the same frames at the protocol boundary
- `ToolTester.tsx` has **one** frame decoder

#### The MCP alignment, restated

This is also why pinning the catalog envelope to MCP shape matters — the `Tool` object the LLM sees via `tools/list`, the operator sees via `/admin/tools`, and `ToolTester` consumes is **one object**. Same `id`, same `input_schema`, same `description`. No translation layer.

#### What it changes in the proposal

- The schema-form package (formerly `starter-ui-schema-form`) is **subsumed into** `starter-ui-tool-tester` — the form is one piece of the tester. If a non-tool consumer needs just the form (a settings panel, maybe), the form widget is re-exported separately.
- M2 ships `<ToolTester />`. `/chat` migrates to use it for tool-use pill expansion in the same milestone (small refactor, kills duplication immediately rather than letting it sit).
- The "extract `useToolDispatchStream` hook" line from v2 is replaced by "extract the whole tester component"; the hook is the headless mode of the same package.

### Frontend stack and reuse

**Reuse `@nube/starter-ui-warehouse-explorer`** for table preview and template query results:
- It's a fork of [frectonz/sql-studio](https://github.com/frectonz/sql-studio), already in the workspace, already re-skinned to rubix tokens
- Already designed to mount inside a host that provides `QueryClient` and theme
- Per its README, "PR 3: mount in rubix shell at `/admin/warehouse-explorer`" is its destination — the admin console can either embed it as a tab or deep-link
- Decision: **deep-link from `/admin/tables/:name` to `/admin/warehouse-explorer?table=…`** rather than embedding. Two reasons: keeps the explorer's `SqlProvider` boundary clean, and avoids duplicating the data-display UX (the explorer's grid + filter + paging is more mature than we'd build inline)

**Schema-driven forms:** the form widget is one piece of `<ToolTester />` (see §"Tool dispatch is one component"). It is also reused independently by the flow editor's node config panel (no dispatch, just edit-and-save), so it's re-exported as a standalone:

```
packages/starter-ui-tool-tester/
  src/
    ToolTester.tsx       # the whole dispatch widget
    SchemaForm.tsx       # JSON Schema → form — re-exported for non-dispatch use
    ...
```

One implementation, three concrete consumers (admin console, chat tool-use pill expansion, flow editor node config), consistent UX across the platform.

## Phasing

**M1 — Read-only catalog + secure foundations (~1.5 weeks)**
- Schema CI gate (blocking)
- `RouteRegistrar` lands, all existing routes migrate to it
- `GET /api/v1/admin/registry` (and per-kind URL sugar) for: tools, nodes, rules, templates, tables, skills, extensions, overview
- `GET /api/v1/admin/openapi.json` from the registrar
- `admin:read` role gated; audit log wired for all `/admin/*` calls
- Frontend: browse-only console + `starter-ui-tool-tester` package scaffolded (component shells, no dispatch yet)
- **MCP alignment check**: confirm `starter-mcp`'s `tools/list` returns the same payload shape

**M2 — Invocation (~1 week)**
- `admin:invoke` role added; tenant-required body validation
- `POST /api/v1/admin/tools/:id/invoke` — streams SSE in the shared frame shape (extracted from `chat_stream.rs`)
- `POST /api/v1/admin/templates/:name/query` with rate limiting
- Table preview: **only if** the RLS-scoped DB role exists; otherwise defer
- **`<ToolTester />` lands** in `packages/starter-ui-tool-tester/`: form + dispatch + streaming response
- `/chat` migrates its `tool_use` pill expansion to use `<ToolTester />` (read-only mode) — kills the duplication immediately rather than letting it sit
- Deep-link from `/admin/tables/:name` into `@nube/starter-ui-warehouse-explorer`

**M3 — Routes + slots (~1 week)**
- `HostSlotRegistry` lands at boot (new infra)
- `/api/v1/admin/routes` + `/admin/slots` (declared + filled)
- Console: route try-it-now (curl-style); slot map view for extension authors

There is no M4. Schema backfill is M1; OpenAPI is M1; warehouse preview is reuse, not build.

## Open questions

- **Does an RLS-scoped DB role exist today for warehouse access?** Blocks the table preview part of M2. If not, M2 ships invoke + template query (tenant-scoped via `Principal`) and table preview slips to a follow-up.
- **Does `starter-audit` exist, or is it the same audit log dependency raised in [flow-storage-and-undo-redo.md §3.3](./flow-storage-and-undo-redo.md)?** Both proposals depend on it. Should be a single audit-log proposal.
- **`HostSlotRegistry` content** — is the slot list declared in code (compile-time const) or in `slots.yaml`? Yaml is more discoverable; code is type-safe. Lean code, but reasonable people disagree.
- **MCP shape compatibility** — confirm `starter-mcp` actually returns the MCP `tools/list` shape today, not a custom one. If custom, M1 picks: change MCP to match, or keep two shapes. Strong preference for one shape.

## Why this matters

The extension architecture is now powerful enough that no human can hold the full surface in their head. Without introspection, the system gets harder to use the more capable it becomes. The admin/test console is the surface that makes the platform self-describing — which unlocks operator productivity, extension authoring, and LLM-driven composition (an agent that can `GET /admin/registry?kinds=tool` is an agent that can compose flows without us hardcoding tool lists in prompts).

Critically, this proposal *no longer* treats `/admin/*/invoke` as a casual addition. It is a tenant-acting, audit-logged, role-gated surface with the same blast-radius discipline as the rest of the platform.
