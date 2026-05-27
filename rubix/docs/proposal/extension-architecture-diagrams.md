# Architecture Diagrams

Companion to [extension-architecture-north-star.md](./extension-architecture-north-star.md).
Two views of the same stack:

1. **Simple view** — the five subsystems most contributors touch.
2. **Full view** — every layer, every middleware, every flavour.

---

## 1. Simple view — the five things that matter

Verified against [rubix/crates/rubix-agent/src/main.rs](../../crates/rubix-agent/src/main.rs) at boot. Boxes are **peers**, not a stack of dependencies. The connective tissue is the **tool registry**: every REST call, every MCP `tools/call`, every flow node, and every extension handler lands on it.

```text
                       ┌──────────────────────────────────┐
                       │             BROWSER              │
                       │  dashboards · SDUI · ext UI (MF) │
                       └──────────────────┬───────────────┘
                                          │ REST / SSE / gRPC + cookie auth
                                          ▼
   ╔════════════════════════════════════════════════════════════════════════╗
   ║                            RUBIX HOST                                   ║
   ╠════════════════════════════════════════════════════════════════════════╣
   ║                                                                         ║
   ║  ┌────────────────────────── INGRESS ─────────────────────────────┐    ║
   ║  │  axum router  +  middleware (principal · authz · changelog ·   │    ║
   ║  │  audit · CORS · CSRF · tenancy · request-id · tracing)          │    ║
   ║  │                                                                 │    ║
   ║  │  /api/v1/{auth, tools/{id}, mcp, ui, flows/{id}/run,            │    ║
   ║  │           dashboards/events, chat/stream, extensions/*}         │    ║
   ║  │  /extensions/{id}/ui/*           (ServeDir, Module-Federation)  │    ║
   ║  │  /api/warehouse/explorer/*       (admin only, no /api/v1)       │    ║
   ║  │  /api/v1/ext/{id}/{rest,sse}/*   (proposed)                     │    ║
   ║  └─────────────────────────────────────────────────────────────────┘    ║
   ║                                  │                                      ║
   ║                                  ▼                                      ║
   ║   ┌───────────────────── COORDINATION LAYER ─────────────────────┐     ║
   ║   │                                                               │     ║
   ║   │   ┌─────────────────┐    ┌──────────────────┐                │     ║
   ║   │   │ TOOL REGISTRY   │◀──▶│  FLOW ENGINE     │                │     ║
   ║   │   │ (the brain)     │    │  scheduler ·     │                │     ║
   ║   │   │                 │    │  nodes · runs ·  │                │     ║
   ║   │   │ rubix-tools +   │    │  flow events SSE │                │     ║
   ║   │   │ ext contribs +  │    │                  │                │     ║
   ║   │   │ MCP surface     │    │ flow→tool        │                │     ║
   ║   │   │                 │    │ (FlowAsTool,     │                │     ║
   ║   │   │                 │    │  proposed name)  │                │     ║
   ║   │   └────────┬────────┘    └────────┬─────────┘                │     ║
   ║   │            │                      │                          │     ║
   ║   │   ┌────────┴────────┐    ┌────────┴─────────┐                │     ║
   ║   │   │  SDUI / DASH-   │    │  AI RUNNER /     │                │     ║
   ║   │   │  BOARDS         │    │  AGENT           │                │     ║
   ║   │   │ pages, schema,  │    │ chat-stream SSE, │                │     ║
   ║   │   │ AnalyticsBridge │    │ MCP back-channel │                │     ║
   ║   │   └─────────────────┘    └──────────────────┘                │     ║
   ║   └────────────────────────────┬────────────────────────────────┘     ║
   ║                                │                                       ║
   ║                                ▼                                       ║
   ║   ┌────────────────────── STORAGE  (peer stores) ──────────────────┐   ║
   ║   │                                                                 │   ║
   ║   │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │   ║
   ║   │  │  POSTGRES    │  │  WAREHOUSE   │  │  BLOBS               │  │   ║
   ║   │  │ (starter-    │  │ (starter-    │  │ (starter-blob-*)     │  │   ║
   ║   │  │  store-      │  │  store-      │  │                      │  │   ║
   ║   │  │  postgres)   │  │  warehouse,  │  │ fs · s3 · garage ·   │  │   ║
   ║   │  │              │  │  Timescale)  │  │ memory · compose     │  │   ║
   ║   │  │ dims · auth  │  │              │  │                      │  │   ║
   ║   │  │ undo · log   │  │ samples ·    │  │                      │  │   ║
   ║   │  │ dashboards   │  │ events ·     │  │                      │  │   ║
   ║   │  │ defs         │  │ documents ·  │  │                      │  │   ║
   ║   │  │              │  │ raw_events   │  │                      │  │   ║
   ║   │  │              │  │ caggs L3     │  │                      │  │   ║
   ║   │  │              │  │  (planned)   │  │                      │  │   ║
   ║   │  └──────────────┘  └──────────────┘  └──────────────────────┘  │   ║
   ║   └─────────────────────────────────────────────────────────────────┘   ║
   ║                                                                         ║
   ║   ┌────────────── CROSS-CUTTING SERVICES (used by all) ───────────┐    ║
   ║   │  secrets · AI providers · i18n · audit/changelog ·            │    ║
   ║   │  observability · cron · insights · alerts · undo · authz ·    │    ║
   ║   │  prefs · tags · tenants                                        │    ║
   ║   └────────────────────────────────────────────────────────────────┘    ║
   ║                                                                         ║
   ╠════════════════════════════════════════════════════════════════════════╣
   ║                                  │                                      ║
   ║                                  │ typed Ctx capabilities               ║
   ║                                  │ (JSON-RPC, supervisor-gated;         ║
   ║                                  │  CallerIdentity stamping — proposed) ║
   ║                                  ▼                                      ║
   ║   ┌───────────────────── EXTENSIONS ──────────────────────────────┐    ║
   ║   │                                                                │    ║
   ║   │  builtin · process · wasm  —  one source, three flavours       │    ║
   ║   │                                                                │    ║
   ║   │  block.yaml declares:                                          │    ║
   ║   │  ├─ contributes : tools · ui · skills · flows · nodes ·        │    ║
   ║   │  │                rest · sse · grpc · mcp                       │    ║
   ║   │  │     (these are merged INTO the host's registries above —    │    ║
   ║   │  │      an ext tool is just a row in TOOL REGISTRY)            │    ║
   ║   │  │                                                              │    ║
   ║   │  └─ capabilities : the host surface the ext consumes           │    ║
   ║   │       (warehouse · postgres · blob · flow_run · ai · audit ·   │    ║
   ║   │        sdui · prefs · authz · http_out · secrets · …)          │    ║
   ║   └────────────────────────────────────────────────────────────────┘    ║
   ╚════════════════════════════════════════════════════════════════════════╝
```

### Why warehouse is **not** central

Earlier drafts put warehouse in the middle and pointed every other box at it. That was wrong. In the actual code:

- **Postgres carries more state than warehouse**: auth, users, prefs, undo, changelog, dashboards definitions, extension lifecycle, dim tables.
- **Warehouse only carries history**: time-series rows in `samples` / `events` / `documents` / `raw_events` (TimescaleDB hypertables). L3 continuous aggregates are planned — `cagg` helpers exist but no caggs are created at migrate time yet.
- **They are peers.** A tool can read either. The coordination layer is what's central, not any one store.

### How the layers connect

| From → To | What flows | Wire |
|---|---|---|
| Browser → Ingress | requests | REST / SSE / gRPC |
| Ingress → Tool Registry | every `/api/v1/tools/{id}` POST | in-process |
| Ingress → MCP | every `tools/call` JSON-RPC at `/api/v1/mcp` | in-process |
| Ingress → SDUI | every page request | in-process |
| Ingress → Extensions admin | `/extensions/*` lifecycle | in-process |
| Tool Registry ↔ Flow Engine | flows-as-tools and tools-in-flows | in-process |
| Tool Registry → Storage | typed sqlx writes/reads | sqlx |
| SDUI → Warehouse | `AnalyticsBridge` named templates | sqlx |
| AI runner → Tool Registry | MCP back-channel to dispatch tools mid-turn | in-process |
| Coordination → Cross-cutting | every subsystem uses secrets / audit / i18n / tracing | in-process |
| **Extensions → Host (any subsystem)** | `ctx.warehouse_read()`, `ctx.flow_run()`, `ctx.audit()`, `ctx.sdui()`, … | typed capability (JSON-RPC stdio) |
| **Host → Extensions** | tool dispatch, node invocation, lifecycle events | JSON-RPC stdio |

### The rules in one line each

1. **Extensions never bind a port** — the host owns every socket and generates routes from `block.yaml`.
2. **Browser ↔ extension always through the host** — REST in, JSON-RPC down, typed reply up.
3. **Extension ↔ host is typed capabilities, not free-form RPC** — every subsystem is reachable, but only by name, only under a declared scope. `CallerIdentity` stamping by the supervisor is proposed (see north-star §"Caller identity propagation") and is the lynchpin of the tenancy-binding story — it must land before any tenant-scoped capability handle ships.

---

## 2. Full view — every layer

For the architecture review. Same picture, more honesty about what's shipped vs proposed.

```text
                                  ┌─────────────────────────────┐
                                  │         BROWSER             │
                                  │  rubix shell (React + Vite) │
                                  │  ├─ admin UI                │
                                  │  ├─ /extensions/* host      │
                                  │  └─ Module-Federation       │
                                  │     loads remoteEntry.js    │
                                  │     from each ext           │
                                  │                             │
                                  │  Host hand-off via          │
                                  │  factory.init(handle):      │
                                  │  ├─ handle.singletons       │
                                  │  │   ├─ react / react-dom   │
                                  │  │   ├─ react-query         │
                                  │  │   └─ zustand             │
                                  │  ├─ handle.hooks            │
                                  │  │   ├─ useHostFormatters   │
                                  │  │   ├─ useHostPreferences  │
                                  │  │   └─ useHostIntl         │
                                  │  └─ handle.client           │
                                  │      (StarterClient w/ auth)│
                                  │                             │
                                  │  Hand-authored, no Vite     │
                                  │  federation plugin needed.  │
                                  │  Charting libs (Recharts,   │
                                  │  ECharts, Plotly) ride MF   │
                                  │  React; bundle size is the  │
                                  │  ext author's problem.      │
                                  └──────────────┬──────────────┘
                                                 │
                          ┌──────────────────────┼──────────────────────┐
                          │                      │                      │
                       REST/JSON              SSE                    gRPC
                       (auth cookie)          (live updates)         (future)
                          │                      │                      │
                          ▼                      ▼                      ▼
   ════════════════════════════════════════════════════════════════════════════════
                         RUBIX HOST  (rubix-agent — single binary)
   ════════════════════════════════════════════════════════════════════════════════

   ┌───────────────────────────────────────────────────────────────────────────────┐
   │                              AXUM HTTP SERVER                                  │
   │                                                                                │
   │  ┌──── public routers (all gated by host middleware) ────────────────────┐    │
   │  │                                                                        │    │
   │  │  /api/v1/auth/*               starter-auth-*                          │    │
   │  │  /api/v1/tools/{id}           ext-server REST dispatcher → JSON-RPC   │    │
   │  │  /api/v1/extensions/*         ext lifecycle / manifest                │    │
   │  │  /api/v1/ext/{id}/rest/*      [proposed] per-ext REST contributions   │    │
   │  │  /api/v1/ext/{id}/sse/*       [proposed] per-ext SSE contributions    │    │
   │  │  /api/v1/mcp                  starter-mcp                             │    │
   │  │  /api/v1/ui/*                 starter-sdui-routes                     │    │
   │  │  /api/v1/dashboards/events    sidebar SSE                             │    │
   │  │  /api/v1/chat/stream          AI streaming SSE                        │    │
   │  │  /api/warehouse/explorer/*    starter-warehouse-explorer (Admin only) │    │
   │  │  /extensions/{id}/ui/*        ServeDir (Module-Federation entries)    │    │
   │  └────────────────────────────────────────────────────────────────────────┘    │
   │                                                                                │
   │  ┌──── middleware stack (every request) ────────────────────────────────┐     │
   │  │   with_principal  →  with_role / with_permission  →  audit  →  CORS  │     │
   │  │   tenancy filter  →  rate-limit  →  request-id  →  tracing            │     │
   │  └───────────────────────────────────────────────────────────────────────┘     │
   └───────────────────────────────────────────────────────────────────────────────┘

   ┌───────────────────────────────────────────────────────────────────────────────┐
   │                       EXTENSION SUPERVISOR  (starter-ext-supervisor)           │
   │                                                                                │
   │  ┌── REST → JSON-RPC dispatcher ──┐    ┌── reverse channel ────────────────┐  │
   │  │                                 │    │                                    │  │
   │  │  HTTP request                   │    │  child JSON-RPC notification      │  │
   │  │  ──▶ frame `tools/{id}` to      │    │  ──▶ capability wire-gate         │  │
   │  │      stdin of process child     │    │      (checks block.yaml grants)   │  │
   │  │      OR call builtin fn         │    │  ──▶ dispatch into typed handle   │  │
   │  │      OR call wasm export        │    │  ──▶ reply down stdin             │  │
   │  └─────────────────────────────────┘    └────────────────────────────────────┘  │
   │                                                                                │
   │  CallerIdentity stamping (PROPOSED — not yet implemented):                     │
   │    { tenant_id, user_id, roles, request_id }                                   │
   │  Supervisor will stamp on every inbound frame; child cannot forge or override. │
   │  Must land in starter-ext-spi + starter-ext-supervisor before any tenant-      │
   │  scoped capability handle (warehouse / postgres / blob / …) ships.             │
   └───────────────────────────────────────────────────────────────────────────────┘

   ┌──── CAPABILITY HANDLES  (the typed extension→host surface) ───────────────────┐
   │                                                                                │
   │   shipped today                          │   proposed (roadmap)               │
   │   ─────────────────                      │   ──────────────────               │
   │   SecretsHandle                          │   WarehouseReadHandle              │
   │   HttpOutHandle    (authority allowlist) │     ├─ named templates only        │
   │   FsHandle         (path allowlist)      │     ├─ tenant_id from caller       │
   │   WallClockHandle                        │     └─ BoxStream<Row>              │
   │   TracingHandle                          │   WarehouseWriteHandle             │
   │                                          │   EventBusHandle                   │
   │                                          │   BlobHandle                       │
   │                                          │   PrefsHandle                      │
   │                                          │   I18nHandle                       │
   │                                          │   AuditHandle                      │
   │                                                                                │
   │   Every handle:                                                                │
   │     • declared in block.yaml `capabilities:` with `v: <major>`                 │
   │     • generated as a typed method on `Ctx` by requires!{} macro                │
   │     • wire-gated by the supervisor (no host_call escape hatch — SCOPE R6)      │
   │     • binds CallerIdentity into the resulting query/filter                     │
   └───────────────────────────────────────────────────────────────────────────────┘

   ┌──── REGISTRIES  (populated at boot from each loaded ext's block.yaml) ────────┐
   │   ToolRegistry      ◀── contributes.tools[]                                    │
   │   SkillRegistry     ◀── contributes.skills[]   (quarantined per R-skills-3)    │
   │   NodeKindRegistry  ◀── contributes.nodes[]    (via ProcessNodeProxy)          │
   │   FlowRegistry      ◀── contributes.flows[]                                    │
   │   UiSlotRegistry    ◀── contributes.ui.exposes[]                               │
   └───────────────────────────────────────────────────────────────────────────────┘

   ┌──── HOST-OWNED STORAGE & SERVICES ────────────────────────────────────────────┐
   │                                                                                │
   │  starter-store-postgres        starter-store-warehouse (Timescale on Pg)       │
   │  ├─ dimensions                 ├─ raw_events    (L1 hypertable)                │
   │  ├─ auth / users / roles       ├─ samples       (L2 hypertable)  ◀── ingest   │
   │  ├─ config / prefs             ├─ events        (L2 hypertable)                │
   │  ├─ undo / changelog           ├─ documents     (L2 hypertable)                │
   │  └─ flow scheduler claims      └─ continuous aggregates (L3 marts, planned)    │
   │                                                                                │
   │  starter-secrets-{file,keyring}    starter-blob-{fs,s3,garage,memory}          │
   │  starter-ai (provider abstraction) starter-i18n                                │
   │  starter-flow-surfaces (scheduler) starter-observability (tracing)             │
   └───────────────────────────────────────────────────────────────────────────────┘

   ════════════════════════════════════════════════════════════════════════════════
                       EXTENSION FLAVOURS  (one source, three flavours)
   ════════════════════════════════════════════════════════════════════════════════

   ┌─── builtin ─────────────┐  ┌─── process ─────────────┐  ┌─── wasm ────────────┐
   │                         │  │                         │  │                      │
   │ statically linked       │  │ separate OS process,    │  │ wasmtime guest,      │
   │ into rubix-agent        │  │ supervisor manages      │  │ fuel + memory        │
   │                         │  │ stdio JSON-RPC          │  │ bounded              │
   │ in-process dispatch     │  │                         │  │                      │
   │ (no framing cost)       │  │ register_process_main!  │  │ wit-bindgen exports  │
   │                         │  │ macro generates run()   │  │                      │
   │ used by:                │  │                         │  │ capability handles   │
   │ ├─ rubix-tools          │  │ used by:                │  │ via wasm imports     │
   │ └─ rubix's own bundled  │  │ ├─ com.rubix.example    │  │ (novel design work   │
   │    skills/tools         │  │ └─ all 3rd-party        │  │  per handle — slower │
   │                         │  │    extensions           │  │  to land than B/P)   │
   └─────────────────────────┘  └─────────────────────────┘  └──────────────────────┘
                  │                          │                          │
                  └──────────────────────────┼──────────────────────────┘
                                             │
                                       ALL share:
                                       ├─ same `Ctx` type generated by requires!{}
                                       ├─ same handler signatures
                                       ├─ same block.yaml contract
                                       └─ same JSON-RPC method names

   ════════════════════════════════════════════════════════════════════════════════
                              ONE EXTENSION  (anatomy)
   ════════════════════════════════════════════════════════════════════════════════

   com.acme.history/
   ├── block.yaml                  ─── manifest: id, version, runtime,
   │                                   contributes:{tools, ui, skills, flows, nodes},
   │                                   capabilities:{warehouse_read, http_out, …}
   │
   ├── kinds/                      ─── JSON schemas for tool I/O + descriptions
   │   ├── list_in.json
   │   ├── list_out.json
   │   └── list.md
   │
   ├── process/                    ─── the Rust binary (process flavour)
   │   ├── Cargo.toml              ─── ONLY dep: starter-ext-sdk  (SCOPE R8)
   │   └── src/main.rs             ─── #[derive(Extension)] struct + handler impls
   │                                   register_process_main!{} → run() loop
   │
   ├── ui/                         ─── federated UI shipped to the browser
   │   ├── remoteEntry.js          ─── Module-Federation entry, registers components
   │   └── main.tsx                ─── developer-facing source
   │
   ├── skills/                     ─── SKILL.md bundles (quarantined by default)
   │   └── example-skill/SKILL.md
   │
   ├── flows/                      ─── YAML flow definitions referencing this ext's
   │   └── example-assistant.yaml      tools/nodes
   │
   └── i18n/                       ─── per-language catalogs (en.json, es.json, …)
                                       auto-prefixed with extension id

   ════════════════════════════════════════════════════════════════════════════════
                  END-TO-END REQUEST  (history page — TARGET STATE)
   ════════════════════════════════════════════════════════════════════════════════
   Steps 3, 5, 6 below depend on three things NOT YET SHIPPED:
     • CallerIdentity stamping (proposed)
     • WarehouseReadHandle             (proposed — see north-star Appendix A)
     • TemplateRegistry wire-gate      (proposed — see north-star Appendix A)
   Today, the same browser→host hop (steps 1–2, 9–10) works against builtin tools;
   the extension side uses HttpOutHandle as the temporary shortcut.


   1. Browser           POST /api/v1/tools/com.acme.history.list  +  cookie
      (HistoryPanel)    body: { params: { entity_id, from, to, limit } }
                                │
   2. Host middleware           ▼
      with_principal  ──▶  resolve user + tenant from cookie
      with_role       ──▶  check tenant has tool access
      audit           ──▶  record { user, tool, params_hash, request_id }
                                │
   3. Tool dispatcher           ▼
      resolve(com.acme.history.list) → process flavour, pid X
      stamp CallerIdentity { tenant_id=A, user_id=…, request_id=r1 }
      frame JSON-RPC → child stdin
                                │
   4. Child process             ▼
      stdio loop receives → handle_com_acme_history_list(ctx, params)
      handler body:
          ctx.warehouse_read().query("samples_window", params)
                                │
   5. Capability call           ▼
      child sends JSON-RPC notification on stdout:
          { method: "warehouse_read.query",
            params: { template, params },
            caller: <opaque ref> }
                                │
   6. Supervisor wire-gate      ▼
      check com.acme.history declares `warehouse_read: { v: 1, tables: [samples] }`
      check template "samples_window" only references declared tables → OK
      bind $caller_tenant_id from CallerIdentity → SQL filter
                                │
   7. Host query layer          ▼
      run "SELECT … FROM samples WHERE tenant_id = $1 AND …" on starter-store-warehouse
      stream rows back as `stream.event` notifications down child's stdin
                                │
   8. Child handler             ▼
      collect stream → Vec<Row> (or pass through as stream)
      return Result<Value>
                                │
   9. Supervisor                ▼
      receive child's JSON-RPC response on stdout
      reply HTTP 200 + body to browser
                                │
  10. Browser                   ▼
      HistoryPanel renders rows through useHostFormatters
      (canonical SI from server → user's unit/locale prefs in browser)
```
