# Proposal: Extension Architecture — North-Star Shape

**Status:** Proposed
**Date:** 2026-05-27
**Author:** NubeDev
**Relates to:**
- `starter-extensions/DOCS/extensions/scope/SCOPE.md` (R6, R8 — the existing extension contract)
- [warehouse-engine-swap.md](./warehouse-engine-swap.md) (Timescale read surface — first real consumer of `WarehouseHandle`)
- `rubix/extensions/com.rubix.example/` (reference layout — what every third-party block follows)

## TL;DR

> **Extensions never bind a port. The host owns every socket the outside world touches and exposes REST / SSE / gRPC on the extension's behalf. Inside the box, host↔extension is JSON-RPC, projected as typed capabilities so the manifest is the contract.**

Three rules. Everything else is filling in capability handles.

This document writes that shape down so all subsequent extension work (history pages, warehouse readers, third-party blocks) lands against the same target.

## Why we need this written down

Today's docs cover the *contract* (`SCOPE.md` R6/R8), the *example layout* (`rubix/extensions/com.rubix.example/README.md`), and individual *transport slices* (REST dispatcher, MCP wrapper, federated UI). They do not write down the **end-to-end topology** in one place, so every new extension author re-derives it — and gets it wrong in the same way: "can my extension expose a REST endpoint?" / "can my child process call back to the warehouse?" / "should the browser talk JSON-RPC to my extension?".

The answer to all three is **no, and here's why**.

## The topology — one picture

```
                    ┌────────────────────────────────────────────┐
                    │              rubix host                     │
                    │                                             │
   browser ──REST──▶│  authz · audit · rate-limit · tenancy       │──JSON-RPC──▶ extension
   browser ──SSE───▶│  routing · CORS · CSRF · TLS · versioning   │  (stdio child /
   client ──gRPC──▶ │                                             │   in-proc builtin /
                    │  exposes REST/SSE/gRPC/MCP on behalf of     │   wasm guest)
                    │  every loaded extension, generated from     │                       │
                    │  block.yaml contributions                   │◀── typed Ctx ─────────┘
                    └────────────────────────────────────────────┘    capabilities
                                                                      (warehouse, secrets,
                                                                       http_out, fs, …)
```

Two wire transports, three flavours, one framing on the inside.

## The three rules

### Rule 1 — Extensions never bind a port

The host owns every socket the outside world touches. An extension that wants a REST endpoint, an SSE stream, a gRPC method, or an MCP tool **declares it in `block.yaml`**; the host materialises the route and routes calls into the extension over the existing JSON-RPC channel.

Already shipping:

| Contribution     | What the host generates                                                                          |
|------------------|--------------------------------------------------------------------------------------------------|
| `tools[]`        | `POST /api/v1/tools/{id}` (SDK router emits `/tools/{id}`; rubix-agent nests it under `/api/v1`) |
| `ui.exposes[]`   | `/extensions/{id}/ui/*` (Module-Federation, served via ServeDir)                                 |
| `skills[]`       | `SkillRegistry` entries (quarantined per SCOPE R-skills-3)                                       |
| `nodes[]`        | `NodeKindRegistry` entries via `ProcessNodeProxy` (shipped, not slice-gated)                     |

To-be-mounted (manifest schema and SDK adapters exist today; just not wired into `rubix-agent`'s boot composer — an afternoon of plumbing, not new SPI design):

| Contribution     | What the host should generate                                                    |
|------------------|----------------------------------------------------------------------------------|
| `rest[]`         | `GET\|POST /api/v1/ext/{id}/rest/<path>` — typed input/output schema             |
| `sse[]`          | `GET /api/v1/ext/{id}/sse/<channel>` — server-sent stream from a typed handler   |
| `grpc[]`         | gRPC service+method names mounted on the host's gRPC server                      |
| `mcp[]`          | Already wrapped: `tools[]` → MCP `tools/<id>` via `starter-ext-mcp::tool_wrapper`|

**Why:** authz, audit, rate-limit, tenancy filtering, version negotiation, CORS, CSRF, TLS are *one* implementation in the host. If extensions opened their own sockets:
- Every author would re-implement (or skip) those concerns.
- Operators would have to trust extension code on the security perimeter.
- We'd lose per-call audit, because the host is the choke-point that emits it.
- The supervisor would lose the ability to kill / restart / sandbox a wedged extension without breaking its public surface.

### Rule 2 — Browser→extension is REST through the host

The browser only ever speaks HTTP/SSE/gRPC to the host's domain. The host translates each request into a JSON-RPC frame down the stdio pipe to the child process (or an in-proc dispatch for builtin, or a wasm import for wasm).

**One framing on the inside, three transports on the outside.** Concretely:

```
Browser                  Host                          Extension child
───────                  ────                          ───────────────
POST /api/v1/tools/X  ──▶ resolve(X) → process flavour
                          │                              stdin frame:
                          ├─ JSON-RPC frame  ──────────▶ { "jsonrpc":"2.0",
                          │                                "method":"tools/X",
                          │                                "params":{…} }
                          │                              dispatch into
                          │                              handle_X(ctx, params)
                          │                              stdout frame:
                          │                            ◀─{ "jsonrpc":"2.0",
                          │                                "result":{…} }
                          ◀─ HTTP 200 + body
```

The wire JSON-RPC is an implementation detail. The browser doesn't know whether `X` is builtin, process, or wasm — and tomorrow we can swap an extension's flavour without touching any frontend code. This is exactly the property "one source, three flavours" exists for (`starter-extensions/SCOPE.md` R1).

### Rule 3 — Extension→host is typed capabilities, not free-form JSON-RPC

When a handler needs the warehouse, secrets, http_out, fs, etc., it calls `ctx.warehouse().query(…)` — a method that exists *only* if `block.yaml` declared `capabilities: [warehouse_read]`.

Under the hood it's still JSON-RPC on the same pipe (notifications from child to host, replies back). The SDK **will not let you write `host_call("anything", params)`** (SCOPE R6).

Current capability set ([`starter-ext-sdk/src/ctx.rs`](../../../starter-extensions/crates/starter-ext-sdk/src/ctx.rs)):

```rust
SecretsHandle      // capabilities.secrets
HttpOutHandle      // capabilities.http_out
FsHandle           // capabilities.fs
WallClockHandle    // capabilities.wall_clock
TracingHandle      // always
```

What an extension declares it consumes:

```yaml
# block.yaml
capabilities:
  http_out:
    authorities: ["https://api.acme.com"]
  secrets:
    prefixes: ["acme.api."]
```

```rust
starter_ext_sdk::requires! {
    name = AcmeCtx,
    capabilities = [http_out, secrets],
}
```

**Why typed and not string-keyed:**
- The manifest is the readable contract. Operators inspecting `block.yaml` know exactly what an extension can reach.
- A new host capability is an additive code change (trait + `Capability` enum variant + supervisor wiring), reviewable in PR.
- The supervisor enforces capability gates at the JSON-RPC wire — a child that tries an undeclared method is killed with `Error::Capability`, not silently dropped.
- Adding `host_call` would make every extension a black box: there is no longer anything to audit at install time.

#### Caller identity propagation (mandatory, not optional)

Every capability call carries an **immutable caller identity** injected by the supervisor — never sourced from the child's params. The shape is:

```rust
struct CallerIdentity {
    tenant_id:  TenantId,        // from the originating HTTP request
    user_id:    Option<UserId>,  // resolved by the host's authn middleware
    roles:      RoleSet,         // resolved by the host's authz layer
    request_id: RequestId,       // for end-to-end audit correlation
}
```

The child cannot forge or override these fields. When the supervisor frames the JSON-RPC `tools/X` call into stdin, it stamps the identity into a reserved envelope slot the SDK exposes as `ctx.caller()`. **Every capability handle that reads or writes tenant-scoped state** (warehouse, postgres, blob, flow runs, prefs, SDUI, dashboards, audit, …) **takes the identity from `ctx.caller()` and binds it into the resulting SQL/filter/scope before issuing the operation**. The child never gets to choose its own tenant.

This closes the obvious tenancy leak: if an extension passes `entity_id="tenant-B.elec.meter-7"` while the request was authenticated as tenant A, the warehouse handle filter `WHERE tenant_id = $caller_tenant_id` clamps the result to tenant A regardless of what the child wrote. Authz is enforced inside the host's pool layer, not in extension code.

This is the same problem shape as user-defined SDUI pages with per-user gating; the resolution is identical (caller-identity-bound SQL composition, never client-trusted).

#### Cross-tenant sharing (resource-owning tenant ≠ caller tenant)

The default "bind `$caller_tenant_id` into every query" rule is *safe by default* but would, on its own, make shared dashboards / shared reports impossible by construction (the caller can never read another tenant's rows, full stop). That's wrong: cross-tenant sharing is a first-class feature, not an exception.

The resolution is: tenant-scoped capability handles resolve **two** tenant ids per call, not one:

- `caller_tenant_id` — from `ctx.caller()`, immutable, identifies *who is asking*.
- `owning_tenant_id` — the tenant that owns the resource being read (the dashboard's tenant, the report's tenant, the warehouse rows' tenant). Default = `caller_tenant_id`. Overridable **only** if `AuthzHandle.can(action, resource)` resolves a cross-tenant grant for the caller against the target resource.

The SQL filter then becomes `WHERE tenant_id = $owning_tenant_id`, not `WHERE tenant_id = $caller_tenant_id`. The grant check happens in the host's authz layer *before* template parameter binding — extensions never see ungated cross-tenant data, and the default path (no grant) is identical to today's strict-isolation behaviour.

A user-defined sharing policy ("share dashboard X with users U1, U2 in tenant B") becomes an authz grant row, not a per-template special case. `WarehouseReadHandle` and `DashboardHandle` MUST take `owning_tenant_id` as an explicit parameter (resolved by the handle, not the child) before either ships — retrofitting it later means re-versioning every tenant-scoped capability.

#### Capability versioning is part of the contract

`block.yaml` declares capabilities by **name + major version**:

```yaml
capabilities:
  warehouse_read: { v: 1, tables: ["samples"] }
  http_out:       { v: 1, authorities: ["https://api.acme.com"] }
```

The SDK's `requires!{}` macro pins the major in the generated `Ctx`. When `WarehouseReadHandle` evolves from `query(template, params)` to `query(template, params, options)`, that is a **v2** — the v1 method stays on `WarehouseReadHandleV1` and old extensions keep working until they opt into v2 in their manifest. The host carries both implementations until v1 is sunset per the standard deprecation window.

This avoids the future-pain of "every shipped extension breaks because we added a parameter". Retrofitting versioning later is painful; we pay the small upfront cost now.

## The capability roadmap (additive)

**Design intent: an extension can reach every host subsystem a builtin tool can.** Extensions are first-class citizens, not warehouse readers. The capability list below is the full host surface, grouped by subsystem. Each row is a single PR: add to `starter-ext-spi::Capability`, add a `*Handle` type in `starter-ext-sdk::ctx`, wire the three flavour backends (`starter-ext-host`, `starter-ext-supervisor`, `starter-ext-wasm`).

The goal isn't to *restrict* what extensions can do — it's to make every cross-subsystem call **typed, declared, audited, and tenant-bound**. The same surface that's available to `rubix-tools` is available to extensions; the only difference is that extensions ask for it explicitly in `block.yaml` so operators can see what they get.

### Critical path (ordering, not alphabetic)

The capability tables below group handles by subsystem. The actual rollout order is determined by **dependency**, not by which subsystem comes first alphabetically. Concretely, validated against the Power-BI-style dashboard extension as the target use case:

| # | Handle / slice | Unlocks | Why this position |
|---|---|---|---|
| 1 | `CallerIdentity` stamping in supervisor | Everything tenant-scoped | Nothing tenant-bound can ship before this. Lynchpin of Rule 3. |
| 2 | `WarehouseReadHandle` + builtin `TemplateRegistry` | Tenant-bound history reads | First real consumer of (1); validates the typed-handle pattern end-to-end. Seed the registry from the four templates already shipping inside `AnalyticsBridge` (see Appendix A) — not a from-scratch design. |
| 3 | `contributes.warehouse_templates[]` | Non-developer chart authoring | Co-requisite of (2), not a follow-on. Without this, every new chart needs a host release — the Power-BI UX dies on contact. |
| 4 | `EventBusHandle` | Push-based cross-filter, fewer round-trips | **Promoted ahead of dashboard persistence.** Fix what *breaks under load* before what's *only ugly*: dashboards-via-loopback works today (ugly but functional), but cross-filter as N HTTP round-trips per click does not scale and is the first thing that breaks at demo load (>10 concurrent viewers). Also lifts the perf ceiling so `DashboardHandle` doesn't ship into a known-broken latency profile. |
| 5 | `DashboardHandle` (or scoped `PostgresHandle`) + `AuthzHandle` with cross-tenant grants | Dashboard persistence, ACLs, sharing | Ship together — the share-with-other-tenant feature requires both. `DashboardHandle` MUST take `owning_tenant_id` as an explicit parameter from day one (see §"Cross-tenant sharing"); retrofitting it means re-versioning. Until this lands, extensions fall back to `HttpOutHandle` → host loopback. |
| 6 | `BlobHandle` + `ExportHandle` | PDF / CSV export | Browser-side CSV works for v1; PDF needs server-side. Defer-able to v1.1. |
| 7 | `CronHandle` | Scheduled refresh | Polish. |

Items 1–3 are the **MVP gate** for any dashboard-authoring extension. Item 4 keeps it usable under load. Item 5 unlocks the share-with-other-tenant killer feature. Items 6–7 are polish. The rest of the tables below sit on later milestones and can land opportunistically as concrete extensions demand them.

Wasm flavour lags per handle as already stated; builtin + process ship together per row.

### Already shipping

| Handle              | What it grants                                                                |
|---------------------|-------------------------------------------------------------------------------|
| `SecretsHandle`     | Read secrets from `starter-secrets-*` under a prefix allowlist                 |
| `HttpOutHandle`     | Outbound HTTP under an authority allowlist                                     |
| `FsHandle`          | Read files under a path allowlist                                              |
| `WallClockHandle`   | Current epoch ms (mockable in tests)                                           |
| `TracingHandle`     | Structured tracing into the host's observability pipeline                      |

### Storage & data

| Handle                  | What it grants                                                            | First consumer                          |
|-------------------------|---------------------------------------------------------------------------|------------------------------------------|
| `WarehouseReadHandle`   | Named-template queries against Timescale (L1/L2/L3) with caller tenancy   | history pages, dashboards                |
| `WarehouseWriteHandle`  | Typed ingest into `samples` / `events` / `raw_events` / `documents`       | feed adapters, importers                 |
| `WarehouseRulesHandle`  | Author retention policies + continuous aggregates (mart.create equivalent)| per-tenant analytics extensions          |
| `PostgresHandle`        | Named-template reads + scoped writes against `starter-store-postgres`     | extensions that own their own dim tables |
| `BlobHandle`            | `put` / `get` / `delete` against `starter-blob-*` under a bucket allowlist | export, report, attachment extensions    |
| `UndoHandle`            | Record + replay reversible operations through `starter-undo`              | any extension that mutates host state    |

### Flow engine

| Handle                  | What it grants                                                            | First consumer                           |
|-------------------------|---------------------------------------------------------------------------|-------------------------------------------|
| `FlowRunHandle`         | Trigger a flow by id with params; receive run-id + completion             | scheduler-driven extensions               |
| `FlowDefineHandle`      | Register / replace flow YAML at runtime (gated by `flow.write` permission)| dynamic-flow extensions, builders         |
| `FlowEventsHandle`      | Subscribe to `flow.tick`, `flow.node.start/end`, `flow.error` streams     | observers, custom dashboards              |
| `NodeRegistryHandle`    | Already shipped via `contributes.nodes[]` + `ProcessNodeProxy`            | extensions adding custom node kinds       |

### Server & external API

| Handle                  | What it grants                                                            | First consumer                            |
|-------------------------|---------------------------------------------------------------------------|--------------------------------------------|
| `RestContributeHandle`  | Declare REST routes at `/api/v1/ext/{id}/rest/*` via `contributes.rest[]` | extensions exposing public endpoints       |
| `SseContributeHandle`   | Declare SSE channels at `/api/v1/ext/{id}/sse/*`                          | streaming extensions (live data, AI)       |
| `GrpcContributeHandle`  | Declare gRPC methods served by the host's gRPC server                     | enterprise integrations                    |
| `McpContributeHandle`   | Already shipped via `contributes.tools[]` + `starter-ext-mcp`             | LLM-tool extensions                        |

### Identity, authz, tenancy

| Handle                  | What it grants                                                            | First consumer                            |
|-------------------------|---------------------------------------------------------------------------|--------------------------------------------|
| `PrincipalHandle`       | Inspect `ctx.caller()` (tenant, user, roles) — **read-only**              | every non-trivial extension                |
| `AuthzHandle`           | Query the policy engine: `can(action, resource)` against the caller       | extensions that gate their own UI          |
| `TenantHandle`          | Resolve tenant metadata (display name, settings, feature flags)           | tenant-aware UI extensions                 |

### AI & agents

| Handle                  | What it grants                                                            | First consumer                            |
|-------------------------|---------------------------------------------------------------------------|--------------------------------------------|
| `AiHandle`              | Invoke `starter-ai` providers (chat, completion, embedding) with budgets  | AI-powered extensions                      |
| `AgentHandle`           | Spawn a starter-ai agent run; receive streamed events                     | assistant / copilot extensions             |
| `SkillsHandle`          | Already shipped via `contributes.skills[]`                                | skill-shipping extensions                  |

### Notifications, events, integrations

| Handle                  | What it grants                                                            | First consumer                            |
|-------------------------|---------------------------------------------------------------------------|--------------------------------------------|
| `EventBusHandle`        | `publish(topic, payload)`, `subscribe(topic) -> Stream`                   | reactor extensions, cross-ext coordination |
| `AlertHandle`           | `send(severity, diagnostic)` through the host's alert pipeline            | monitoring extensions                      |
| `InsightsHandle`        | Record insight rows that feed dashboards / sidebar                        | observability extensions                   |
| `ServiceHandle`         | Talk to `starter-service-slack` / `-telegram` / etc. through host routing | notification extensions                    |

### UI surface, i18n, prefs

| Handle                  | What it grants                                                            | First consumer                            |
|-------------------------|---------------------------------------------------------------------------|--------------------------------------------|
| `UiSlotHandle`          | Already shipped via `contributes.ui.exposes[]` (Module-Federation)        | every UI extension                         |
| `I18nHandle`            | `t(key, params)`, formatters — promote today's hook-based API into `Ctx`  | server-rendered prose (alerts, reports)    |
| `PrefsHandle`           | Read user/tenant prefs; subscribe to changes                              | personalised extensions                    |
| `SduiHandle`            | Author / read SDUI pages programmatically                                 | dashboard-builder extensions               |
| `DashboardHandle`       | Read dashboard schema + history (revision diff, restore)                  | dashboard-management extensions            |

### Audit, lifecycle, ops

| Handle                  | What it grants                                                            | First consumer                            |
|-------------------------|---------------------------------------------------------------------------|--------------------------------------------|
| `AuditHandle`           | `record(kind, fields)` into the host's audit log                          | privileged extensions                      |
| `CronHandle`            | Schedule recurring callbacks via `starter-cron`                           | periodic extensions                        |
| `ConfigHandle`          | Read host-wide config under an allowlist                                  | environment-aware extensions               |
| `ExportHandle`          | Trigger `starter-export` jobs (CSV, PDF, JSON)                            | report extensions                          |

None of these is a new transport. All of them ride the existing JSON-RPC pipe; they're just **typed slots in `Ctx`** generated by `requires!{}`.

**The principle: anything the host can do, an extension can ask to do — by name, with a version, under a declared scope, with caller identity bound automatically.** That's what makes extensions first-class. The capability layer is the *contract surface*, not a fence around a sandbox.

Cost-per-handle is honest only for **builtin + process flavours** (~200 lines of glue each). The **wasm** flavour has not yet executed its first non-trivial capability call end-to-end; expect each new handle to require novel design work on the wasm side (component-model bindings, fuel accounting for the host round-trip, cancellation propagation across the wasm boundary). The roadmap PRs should land builtin+process first and treat wasm as a follow-up per handle, not a blocking part of the same PR.

## What this means for a history-page extension (worked example)

> **NB — this example uses `WarehouseReadHandle`, which is proposed (see roadmap), not shipped.** It shows the *target* shape so the rest of the design is concrete. For what works today, see ["The temporary shortcut"](#the-temporary-shortcut-today-before-warehousereadhandle-lands) below.

The concrete shape of the first extension that exercises Rule 3 against the warehouse:

```yaml
# block.yaml
id: com.acme.history
v: 1
runtime: { kind: process, bin: acme-history }
capabilities:
  warehouse_read: { tables: ["samples", "events"] }    # new — Rule 3
contributes:
  tools:
    - id: com.acme.history.list
      input_schema:  kinds/list_in.json
      output_schema: kinds/list_out.json
  ui:
    entry: ui/remoteEntry.js
    exposes:
      - { name: HistoryPanel, module: ./HistoryPanel, slot: main }
```

```rust
// process/src/main.rs
starter_ext_sdk::requires! {
    name = AcmeCtx,
    capabilities = [warehouse_read],
}

impl AcmeHistoryToolHandlers for Acme {
    type Ctx = AcmeCtx;

    async fn handle_com_acme_history_list(
        &self,
        ctx: &Self::Ctx,
        params: Value,                         // { entity_id, from, to, limit }
    ) -> Result<Value> {
        // ctx.caller() carries the supervisor-stamped tenant_id;
        // the handle binds it into the SQL filter automatically.
        let mut stream = ctx.warehouse_read().query(
            "samples_window",
            params,
        )?;
        let mut out = Vec::new();
        while let Some(row) = stream.next().await {
            out.push(row?);
        }
        Ok(json!({ "rows": out }))
    }
}
```

```js
// ui/remoteEntry.js — HistoryPanel
fetch("/api/v1/tools/com.acme.history.list", {
  method: "POST",
  credentials: "same-origin",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ params: { entity_id, from, to, limit: 500 } }),
})
```

End-to-end:

1. Browser → host (`POST /api/v1/tools/com.acme.history.list`) — host authz/audit middleware applies.
2. Host → child (JSON-RPC `tools/com.acme.history.list` on stdio).
3. Child → host (JSON-RPC `warehouse_read.query` on stdio) — supervisor checks the capability gate.
4. Host runs the SQL through the pool, returns rows.
5. Child returns the rows back up.
6. Host returns 200 + body to the browser.

No socket on the extension. No `host_call`. No direct DB access from the child. **Three rules, satisfied.**

## The temporary shortcut (today, before `WarehouseReadHandle` lands)

`HttpOutHandle` already exists. Declare:

```yaml
capabilities:
  http_out:
    authorities: ["http://localhost:3030"]   # rubix-agent loopback
```

Then from the handler:

```rust
let body = ctx.http().request(json!({
    "method": "POST",
    "url": "http://localhost:3030/api/warehouse/explorer/query",
    "body": { "sql": "SELECT ts, value_num FROM samples WHERE entity_id = $1 AND ts >= $2 ORDER BY ts DESC LIMIT 500" },
}))?;
```

Semantically the same shape (typed capability, manifest-declared, supervisor-enforced authority allow-list), routed through HTTP instead of a dedicated verb. Migrate to `ctx.warehouse_read().query(…)` when the typed handle lands — the public tool surface (`/api/v1/tools/com.acme.history.list`) does not change.

> **Caveat — `HttpOutHandle` loopback authorities are a soft trust boundary today.** An extension granted `http_out: ["http://localhost:3030"]` can reach *every* host route at that origin, not just `/api/warehouse/explorer/*`. The cookie-bound authz middleware still gates each route, but the extension can probe the full surface. Treat loopback authorities as a trusted-extension-only shortcut; for third-party extensions, wait for the typed handle. A future `HttpOutHandle` v2 should accept per-authority path allow-lists (`{ authority, paths: ["/api/warehouse/explorer/*"] }`) to tighten this — tracked separately from the dashboard work.

## What this proposal explicitly is *not*

- It does not propose changing the existing `block.yaml` shape — every contribution slice listed in Rule 1 is additive.
- It does not propose deprecating `/api/warehouse/explorer/*` — that surface stays as the **admin** read plane; per-extension reads go through their own tool verbs gated by per-extension capabilities. Two different audiences, two different gates.
- It does not propose a `host_call` escape hatch. SCOPE R6 stays.
- It does not change the federated-UI mechanism. Module-Federation works; nothing in here touches it.

## Acceptance — when this is "done"

This proposal lands the **shape**; the work breaks into one PR per capability handle, each independently shippable. Acceptance for the shape itself:

1. This document is merged under `rubix/docs/proposal/` and referenced from `starter-extensions/DOCS/extensions/SCOPE.md` as the authoritative end-to-end picture.
2. `rubix/extensions/com.rubix.example/README.md` adds a "what extensions can and cannot do" section pointing here.
3. The capability roadmap table becomes a tracking list (one issue per row) with `WarehouseReadHandle` as the first item.

Subsequent PRs (one per row) flip the table entries from "proposed" to "shipped" as each handle lands.

## Risks and counter-arguments

| Concern | Resolution |
|---|---|
| "Why not let extensions bind their own sockets behind a reverse proxy?" | Reverse-proxy config becomes per-extension state operators have to manage. Auth headers, CORS rules, TLS termination all get re-implemented. The host already owns all of it; reusing it is cheaper than duplicating it. |
| "JSON-RPC over stdio is a bottleneck for streaming." | The stream sub-protocol (post-R13) handles this — `stream.event` notifications carry chunks with backpressure via bounded mpsc. SSE on the outside maps cleanly to `EventSender` on the inside (`starter-ext-sdk::ctx::Event`). |
| "Typed capability handles are slow to add." | ~200 lines of glue *per handle* for builtin+process. Wasm is more (novel work per handle until component-model bindings stabilise). Still strictly cheaper than the audit cost of a `host_call` escape hatch. |
| "What about extensions that legitimately need to listen on a port (e.g. a SIP gateway)?" | Out of scope for v0.1. The escape hatch when it arrives is a `network_listen` capability with an explicit port allow-list, supervisor-managed lifecycle, and per-port authz — *not* "extension binds whatever it likes". |
| **Host process is the bottleneck for every external request and every capability call.** | Acknowledged scaling axis. The funnel is the security story — we accept it for now. Future relief paths (in order of likely landing): (1) capability handles that don't touch DB run with zero contention; (2) `WarehouseReadHandle` uses streaming results so a slow consumer doesn't pin a host worker; (3) the supervisor can be sharded per-extension-affinity if a single child saturates the host; (4) as a last resort, "co-located host" mode runs a dedicated host alongside a heavy extension. None of these change the contract. |

## Appendix A — `WarehouseReadHandle` design sketch

This is the design the roadmap row entitled "WarehouseReadHandle" must satisfy before it ships. Calling it out here so the roadmap is not just a name on a list.

### Templates are server-defined, not client-supplied

The handle is **not** a SQL gateway. The host ships a registry of named templates, each with a typed parameter schema and a fixed SQL body. Extensions reference templates by name; they cannot author SQL. Example:

```rust
TemplateRegistry::builtin()
    .with("samples_window", TemplateSpec {
        params: schema!({ entity_id: String, from: Ts, to: Ts, limit: u32<=10_000 }),
        sql: "SELECT ts, value_num, quality, tags \
              FROM samples \
              WHERE tenant_id = $caller_tenant_id \
                AND entity_id = $entity_id \
                AND ts >= $from AND ts < $to \
              ORDER BY ts DESC \
              LIMIT $limit",
        tables: &["samples"],
    })
    .with("events_kind", /* … */)
```

The `$caller_tenant_id` placeholder is bound from `ctx.caller()` by the host — extensions cannot override it. The `tables` field is what `capabilities.warehouse_read.tables: ["samples"]` enforces: a manifest declaring `tables: ["samples"]` cannot invoke a template that touches `events`. The check is at the supervisor wire-gate, not in the child.

**Why server-defined:** every alternative (parameterised SQL fragments, client-templated DSL, full SQL gateway) reopens the audit problem the typed-capability rule exists to close. A library of named templates is reviewable, indexable, and gives operators a finite list to authz against.

**Seed the registry from `AnalyticsBridge`, don't design it from scratch.** `rubix-agent/src/sdui/analytics_bridge.rs` already ships four hard-coded named templates (`meter_kwh_last_24h`, `meter_litres_last_24h`, `meter_value_30d_15m`, `meter_value_24h_1m`) used by the host's own SDUI chart rendering. These are exactly the `TemplateSpec` shape above. The v1 PR is *lift the four templates into `TemplateRegistry::builtin()` and route `AnalyticsBridge` through the registry*, not *design + implement a new subsystem*. The capability handle then wraps the same registry the host already trusts — extensions and the host's own dashboards become consumers of one surface, which is exactly the "extensions are first-class citizens" claim made tangible.

### `contributes.warehouse_templates[]` is co-requisite, not follow-on

Server-defined templates without an extension-side contribution slice means **every new chart requires a host release**. That kills any user-facing dashboard-authoring product on contact. The roadmap reflects this: `contributes.warehouse_templates[]` ships with `WarehouseReadHandle`, not after it.

Third-party extensions can ship their own templates in `block.yaml`, gated by the same operator install step that gates `contributes.nodes[]` and `contributes.skills[]`. The install-time review is the audit point — once installed, the templates are first-class members of the registry and queryable by name like any builtin.

```yaml
# block.yaml
contributes:
  warehouse_templates:
    - name: acme_kpi_window
      params: { tenant: String, kpi: String, from: Ts, to: Ts }
      tables: [samples]
      sql: |
        SELECT time_bucket('1h', ts) AS bucket, avg(value_num) AS v
        FROM samples
        WHERE tenant_id = $owning_tenant_id
          AND entity_id = $kpi
          AND ts >= $from AND ts < $to
        GROUP BY bucket ORDER BY bucket
```

### Calculated fields are template parameters, not free expressions

The template-only model deliberately forbids client-supplied SQL — which forecloses Power-BI-style "calculated measures" as free expressions. The resolution is to model measures as **named template parameters with a closed expression grammar** (a small algebra of `sum/avg/min/max/count/ratio` over allowed columns), not as arbitrary user code. The grammar is reviewable; the template binds the expression into a fixed projection slot. This buys 90% of the calculated-field use case without reopening the SQL-gateway audit problem.

Out of scope for the v1 `WarehouseReadHandle` PR; tracked separately so it doesn't gate the rollout.

### Streaming, not `Vec<Row>`

The handle returns `Stream<Item = Result<Row>>`, not `Vec<Row>`. A 90-day history pull is a memory cliff otherwise. The streaming substrate (`stream.event` notifications, bounded mpsc backpressure) already exists in `starter-ext-spi::jsonrpc`; this handle uses it.

```rust
pub trait WarehouseReadHandle: Send + Sync {
    fn query(
        &self,
        template: &str,
        params:   serde_json::Value,
    ) -> Result<BoxStream<'static, Result<Row>>>;

    fn count(
        &self,
        template: &str,
        params:   serde_json::Value,
    ) -> Result<u64>;
}
```

For REST callers that want a list (not a stream), the host's REST→JSON-RPC dispatcher collects the stream into a JSON array up to a configurable cap (default 10k rows) and returns 413 + a `streaming_endpoint` URL pointer past it. For SSE callers (`contributes.sse[]`), every yielded row becomes an SSE event with no buffering.

### Units / i18n / prefs coupling

The handle returns **canonical-unit numeric values** (SI units, UTC timestamps). It does **not** apply user unit preferences. That coupling stays at the rendering layer (the federated UI's `useHostFormatters`), where it already lives. Reasons:

- Per-user preference resolution at query time would couple every capability handle to the prefs system.
- The canonical-unit payload is cacheable across users; a preference-applied payload is not.
- The frontend already owns the unit-formatting contract (see `DOCS/extensions/guides/i18n.md`); duplicating it server-side risks divergence.

Extensions that want server-rendered prose (alerts, reports) take a separate `I18nHandle` and call `t(key, params)` on it, passing the canonical values. That keeps formatting in one layer.

### Open questions for the WarehouseReadHandle PR

1. Template versioning — same major+minor scheme as capabilities, or a separate registry version?
2. Does `count(template, params)` need its own template entries, or can it derive from the `query` template by rewriting the projection?
3. How does `EXPLAIN`-driven cost-rejection interact with the per-extension fairness budget? (Probably out of scope for v1.)

These are tracked with the implementation PR, not blocking this proposal.

## References

- `starter-extensions/DOCS/extensions/scope/SCOPE.md` — R1 (one source, three flavours), R6 (no `host_call`), R8 (SDK-only dependency).
- `starter-extensions/crates/starter-ext-sdk/src/ctx.rs` — the typed capability handles as they exist today.
- `starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs` — the wire format.
- `starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs` — the REST → JSON-RPC bridge for process-flavour tools.
- `rubix/extensions/com.rubix.example/` — the canonical layout this architecture supports.
- [warehouse-engine-swap.md](./warehouse-engine-swap.md) — phase 4 restored the admin read surface; `WarehouseReadHandle` is the per-extension equivalent.
