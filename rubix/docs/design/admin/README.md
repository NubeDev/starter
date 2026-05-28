# ADMIN — introspection surface

The admin surface is one HTTP namespace, `/api/v1/admin/*`, that
projects every in-process registry the rubix-agent boots — tools,
flow node-kinds, anomaly rules, warehouse templates and tables,
skills, extensions — onto a single uniform wire envelope.

> Cites: SCOPE [R1](../../../SCOPE.md#r1) (verb-per-file layout),
> [R5](../../../SCOPE.md#r5) (transport carries no domain),
> [R6](../../../SCOPE.md#r6) (one wire envelope across surfaces).

## Why this surface exists

Operators landing on a fresh rubix-agent had no built-in way to ask
"what does this binary advertise?". `GET /api/v1/extensions` dumps
manifest blobs; `POST /api/v1/tools/{id}` dispatches but does not
list. Every other registry — node-kinds, rules, templates, tables,
skills — was unreachable over HTTP. The admin surface closes that
gap with **one** projection per registry, all funnelled through one
envelope, all gated by the same role check.

The same envelope is also the contract MCP `tools/list`,
`/api/v1/chat`, and the in-browser test console consume — see
[../mcp-ux/README.md](../mcp-ux/README.md). The admin projection is
the canonical source; the other surfaces are filtered views.

## Wire envelope

Every list returns:

```json
{
  "items": [
    {
      "id": "rubix.warehouse.ingest",
      "label": "Warehouse ingest",
      "summary": "…",
      "source": { "kind": "builtin" },
      "input_schema": { },
      "output_schema": null,
      "metadata": { }
    }
  ],
  "next_cursor": null
}
```

The fields:

| Field | Type | Notes |
|---|---|---|
| `id` | string | Stable item identifier. Tool ids, node kinds, rule ids, template names, table names, skill ids. |
| `label` | string | Short human label. Falls back to `id` when no display name is declared. |
| `summary` | string | One-sentence description. Empty when none declared. |
| `source` | tagged union | `{"kind":"builtin"}` for host-owned, `{"kind":"extension","id":"<rdns>"}` for extension-contributed, `{"kind":"starter"}` for upstream-starter crate-owned. |
| `input_schema` | nullable JSON Schema | Present for kinds that accept structured input (tools, templates, nodes). `null` for kinds where the concept doesn't apply (extensions, skills) or where the item has not declared one (a CI gate exists to push that count to zero — see [§Schema discipline](#schema-discipline)). |
| `output_schema` | nullable JSON Schema | Same shape as input. Tools may declare it; today most do not. |
| `metadata` | object | Per-kind extras. See [§Per-kind metadata](#per-kind-metadata). |

Cursors are opaque base64 strings; default page size 50, maximum 200.

The shape deliberately matches MCP's `tools/list` payload so the MCP
adapter and the admin console consume one projection of one model.

## Endpoints

```
GET  /api/v1/admin/registry?kinds=tool,node,rule,template,table,skill,extension
                            &source=builtin|extension:<id>|starter
                            &limit=50&cursor=…
GET  /api/v1/admin/registry/tools         GET /api/v1/admin/registry/tools/{id}
GET  /api/v1/admin/registry/nodes         GET /api/v1/admin/registry/nodes/{kind}
GET  /api/v1/admin/registry/rules         GET /api/v1/admin/registry/rules/{id}
GET  /api/v1/admin/registry/templates     GET /api/v1/admin/registry/templates/{name}
GET  /api/v1/admin/registry/tables        GET /api/v1/admin/registry/tables/{name}
GET  /api/v1/admin/registry/skills        GET /api/v1/admin/registry/skills/{id}
GET  /api/v1/admin/registry/extensions    GET /api/v1/admin/registry/extensions/{id}
GET  /api/v1/admin/overview                    counts only; cheap
GET  /api/v1/admin/openapi.json                catalog projection of every rubix-agent-owned route
POST /api/v1/admin/registry/tools/{id}/invoke        synchronous dispatch; scope `admin:invoke`
POST /api/v1/admin/registry/tools/{id}/invoke/stream SSE dispatch; same scope, same body, same audit
```

The multiplexed `/admin/registry?kinds=` returns a `RegistrySnapshot`
— one envelope per requested kind, keyed by kind. The per-kind URLs
are exact aliases for `kinds=<one>`; they exist so curl and the
console can deep-link.

### Invocation (`POST /admin/registry/tools/{id}/invoke`)

Synchronous JSON-in / JSON-out dispatch through the same `Tool::invoke`
path `POST /api/v1/tools/{id}` uses, scoped behind an explicit
role + scope gate:

- Body: `{ "tenant": "<tenant-id>", "input": { … } }`.
- `tenant` is **required and non-empty**. Missing, blank, or
  whitespace-only values return `400 bad_request` with an explicit
  message — the admin's session principal is the *actor*, never
  the dispatch tenant.
- The handler scopes a `CallerIdentity` task-local with
  `tenant_id = body.tenant` and `user_id = principal.subject`, so
  extension-backed tools (`ProcessExtensionToolBinding`) call as
  the target tenant rather than the admin's own session.
- Unknown tool id → `404`; tool error → mapped to the matching HTTP
  status via the same `Error → StatusCode` table the
  `/api/v1/tools` router uses (`Invalid → 400`, `Forbidden → 403`,
  `Conflict → 409`, `Internal → 500`).
- Every invoke emits a structured `tracing::info!` line with target
  `rubix.admin.invoke` carrying actor, target tenant, tool id,
  status, and latency. Persistent audit also lands: the router
  sits behind the shared `middleware::changelog_layer` that powers
  `/api/v1/tools/*`, configured with both path prefixes
  (`/api/v1/tools/` and `/api/v1/admin/registry/tools/`). Each
  successful invoke writes one `Change` row keyed on
  `tool.invoke` + the path's tool id, attributed to the admin's
  authenticated subject; the captured payload includes the
  `tenant` field so SIEM consumers can join audit rows to
  tenant-scoped workloads without re-parsing the URL.

### Streaming invoke (`POST /admin/registry/tools/{id}/invoke/stream`)

Same body shape, same role + scope gate, same persistent audit as
the synchronous sibling — the only difference is the wire: frames
flow as Server-Sent Events in the shared
[`StreamFrame`](../../../crates/rubix-agent/src/routes/stream_frames.rs)
shape that the chat surface also emits.

Frames emitted, in order:

1. `connected { model: null }` — opens the stream so a client
   observes the connection before the runner starts (mirrors
   chat).
2. On success: `result { value }` then `done { status: "ok",
   latency_ms }`.
3. On failure: `error { message }` then `done { status, latency_ms }`
   where `status` is one of `ok | not_found | invalid |
   unauthenticated | forbidden | conflict | internal | other` —
   the same set the structured `tracing::info!` audit line uses.

The shared `StreamFrame::Done` variant carries both the chat token
/ cost keys and the admin invoke status / latency keys, each
`Option<...>` with `skip_serializing_if`, so a single client
decoder switches on `type` and plucks what each surface emits.
Today every tool implements `Tool::invoke` synchronously, so the
handler awaits the sync result and translates it into the frame
sequence above; a future `Tool::invoke_stream` (or sidecar
`StreamingTool` trait) lets long-running tools emit progress
frames in the middle without changing the wire shape.

The matching `POST /admin/registry/templates/{name}/query` ships
with the warehouse-query milestone — it needs an RLS-scoped DB
role that the agent does not own today.

## Per-kind metadata

| Kind | `metadata` keys |
|---|---|
| `tool` | `mcp_compatible` (bool — always `true` today), `tags` (vec<string>) |
| `node` | `facets` (vec<string>), `streaming` (bool) |
| `rule` | `priority` (i32 or null), `quality` (string) |
| `template` | `tables` (vec<string>), `sql_preview` (string, ≤200 chars; null for builtin templates without an `sql` body) |
| `table` | `columns` (vec<{name, type, default?}>), `order_by` (vec<string>), `engine` (string or null), `partition_by` (string or null), `ttl` (string or null) |
| `skill` | `quarantined` (bool), `bundle_dir` (string) |
| `extension` | `version` (string), `state` (string: `validated`/`failed`), `contributes` (counts per kind: `{ tools, nodes, rules, templates, tables, skills }`) |

`metadata` is `serde_json::Value` on the wire so per-kind keys can
grow without a `starter-spi` bump. New keys are additive.

## Roles

| Role / scope | Grants |
|---|---|
| `Role::Admin` | Every `GET /api/v1/admin/*` route. |
| `Role::Admin` + scope `admin:invoke` | `POST /api/v1/admin/registry/tools/{id}/invoke`. The scope is checked in addition to the role so an admin who only browses the catalog cannot fire tools. |

The read router (`admin_router`) and the invoke router
(`admin_invoke_router`) are built independently in
[`crate::routes::admin`](../../../crates/rubix-agent/src/routes/admin/mod.rs)
so `main.rs` can layer different gates on each. Both sit inside one
`with_principal(...)` envelope; the invoke router additionally
goes through `with_scope("admin:invoke")`.

A further `admin:read` scope (a junior-operator read-only token
strictly below `Role::Admin`) is documented as a future split; the
current posture lets every admin browse, and gates only the
blast-radius operation. See [../auth/](../auth/README.md).

## Pagination

Cursors are base64 of the last item's `id`. The next page is the
slice strictly after that id in the kind's deterministic order
(every projector emits items sorted by id). When fewer items than
`limit` remain `next_cursor` is `null`.

`limit` defaults to 50 and clamps at 200; out-of-range values
return 400.

## Schema discipline

Items where `input_schema` is `null` because the registry entry
declared none are tracked as a backfill list. The CI gate that
fails on missing schemas is opt-in per kind today (tools) and grows
as each registry's items declare structured input. See
[../tools/README.md](../tools/README.md#schemas) for the tool-side
expectation and [./schemas.md](./schemas.md) for the cross-kind
ledger.

## Layering

The admin surface is transport-only. Every handler does four
things: parse query → look up the in-process registry handle on
state → call a `*_items(...)` projection in `rubix_agent::admin` →
apply pagination → return the envelope. No SQL, no warehouse hits,
no tool dispatch. Projections live in
[crates/rubix-agent/src/admin/](../../../crates/rubix-agent/src/admin/);
routes live in
[crates/rubix-agent/src/routes/admin/](../../../crates/rubix-agent/src/routes/admin/).

## What this surface is not

- **Not a discovery wire for end-user UIs.** The frontend test
  console uses it, but tenant-facing surfaces consume their own
  per-domain endpoints (chat, dashboards, flows).
- **Not a permission engine.** Role-gating is coarse; richer
  per-item authorisation belongs in `starter-authz`.

## OpenAPI projection and route registrar discipline

`/api/v1/admin/openapi.json` is **projected** from the live route
catalog at boot — not authored. Every rubix-agent-owned route is
mounted through one chokepoint,
[`RouteRegistrar`](../../../crates/rubix-agent/src/routes/registrar.rs),
which records a `RouteEntry { method, path, description, tags,
request_schema, response_schema }` parallel to the axum `Router`.
After every merge `main.rs` calls
`routes::catalog_to_openapi(app.catalog(), …)` and mounts the
resulting JSON at the admin path. The projection cannot drift from
the live router because they share one source of truth.

A workspace discipline test
([`route_registrar_discipline_test`](../../../crates/rubix-agent/tests/route_registrar_discipline_test.rs))
fails CI if any `.rs` file outside the registrar contains a raw
`.route(` call — every new rubix-agent route must declare its
metadata via `RouteRegistrar::mount(…)`. Upstream routers
(`starter-auth-users`, `starter-ext-server`,
`starter-warehouse-explorer`, `starter-sdui-routes`, MCP) come in
via `.merge_external(router)` and intentionally bypass the
catalog — they own their own OpenAPI surface.
