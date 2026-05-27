# Extensions — North-Star progress

> **Tier:** scope (plan). Lifetime: weeks–months. Per
> [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md) **source code must
> not reference this file.** Status only — the design lives in
> [README.md](./README.md) and the canonical reference is
> [docs/design/extensions/](../../design/extensions/).

## How to update this doc

Per [NEW-SESSION.md](../../../NEW-SESSION.md): at the end of
each session that moves a row, the session adds a dated entry to
the [Session log](#session-log) and bumps the row's **Status**
in the [Critical-path tracker](#critical-path-tracker).

Status vocabulary (one of):

| Status | Meaning |
|---|---|
| ❌ not started | No code. Manifest field absent, capability absent, no PR open. |
| 🟡 in progress | A PR is open or branch exists; not merged. |
| 🟣 design landed | Subsection of `docs/design/extensions/` exists and matches an in-flight PR. |
| ✅ shipped | Capability handle (builtin + process) merged to `master`; design doc current; tests passing. |
| 🔵 shipped (wasm) | Wasm backend for an already-shipped handle is merged. |

A row stays at ✅ until its wasm follow-up lands; only then does
the row drop off the tracker entirely.

## Critical-path tracker

Phases mirror [README.md §"Critical path"](./README.md).

### Phase 1 — MVP gate

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| 1 | `CallerIdentity` stamping in supervisor | ✅ shipped | — | (master) | [caller-identity.md](../../design/extensions/caller-identity.md) | 2026-05-27 |
| 2 | `WarehouseReadHandle` + builtin `TemplateRegistry` (seeded from `AnalyticsBridge`) | ✅ shipped | — | (master) | [warehouse-read.md](../../design/extensions/warehouse-read.md) | 2026-05-27 |
| 3 | `contributes.warehouse_templates[]` manifest slice | ✅ shipped | — | (master) | [warehouse-templates-contribution.md](../../design/extensions/warehouse-templates-contribution.md) | 2026-05-27 |

### Phase 2 — Usable under load

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| 4 | `EventBusHandle` | ✅ shipped (publish; subscribe deferred) | — | (master) | [event-bus.md](../../design/extensions/event-bus.md) | 2026-05-27 |

### Phase 3 — Sharing

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| 5 | `DashboardHandle` + `AuthzHandle` with cross-tenant grants | ❌ not started | — | — | — | — |

### Phase 4 — Polish

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| 6 | `BlobHandle` + `ExportHandle` | ❌ not started | — | — | — | — |
| 7 | `CronHandle` | ❌ not started | — | — | — | — |

### Plumbing (independent)

| # | Item | Status | Owner | PR(s) | Design doc | Last touched |
|---|---|---|---|---|---|---|
| P1 | Mount `rest[]` / `sse[]` / `workers[]` / `cli[]` adapters in `rubix-agent` boot composer | ❌ not started | — | — | — | — |
| P2 | `HttpOutHandle` v2 — per-authority path allow-lists | ❌ not started | — | — | — | — |

## Gate checks

Phase progression is gated. Do not start a later phase before
the earlier one's gate passes.

- **Phase 1 gate:** all of rows 1, 2, 3 at ✅. A handwritten
  test extension performs a tenant-scoped warehouse read via
  `ctx.warehouse_read().query("samples_window", …)` and
  observes the tenancy clamp from `ctx.caller()`. The
  `HttpOutHandle` → loopback shortcut in any reference
  extension is deleted.
- **Phase 2 gate:** row 4 at ✅. A test extension drives a
  cross-chart filter through `EventBusHandle` with zero
  additional HTTP round-trips per click. Load test: 10
  concurrent viewers × 20 charts × 1 filter change/sec
  sustained for 60s without queue growth in the supervisor.
- **Phase 3 gate:** row 5 at ✅. A dashboard created in tenant
  A is readable by user U2 in tenant B *only* when an
  `AuthzHandle` grant exists; `WarehouseReadHandle` queries
  triggered by U2 filter by tenant A (the owning tenant), not
  tenant B (the caller tenant).
- **Phase 4 gate:** rows 6, 7 at ✅. Power-BI MVP demo path
  ends with "schedule a daily PDF export to a blob bucket" and
  it works.

## Open questions

Carried from [README.md §"Open questions"](./README.md). Move
to **resolved** with a date and the resolution when closed.

| Question | Status | Resolution |
|---|---|---|
| Template versioning scheme | open | — |
| `count()` derived vs own template | open | — |
| `EXPLAIN` cost-rejection + fairness budgets | deferred | — |
| Cross-tenant grant table shape in `starter-store-postgres` | open (gates Phase 3) | — |
| Closed-grammar calculated-fields engine | deferred (post-MVP) | — |

## Session log

Newest entry first. One entry per session that touches this
scope. Format:

```
### YYYY-MM-DD — <one-line session summary>

- Rows moved: <e.g. "row 1: ❌ → 🟡 (PR #123 opened)">
- Decisions: <bullets — open-question resolutions, scope changes, etc.>
- Next: <what the next session should pick up>
```

Keep entries terse. The session log is the audit trail; the
tracker tables above are the source of truth for current state.

---

### 2026-05-27 — row 4 scaffolding: `EventBusHandle` (publish only)

- Rows moved: row 4: ❌ → ✅ shipped (publish; subscribe deferred).
- Changes:
  - `starter-ext-spi`: new `event_bus` module with
    `EventBusMessage { topic, payload, ts_unix_ms }`,
    `EventBusPublishRequest`, `EventBusSubscribeRequest`;
    `Capability::EventBus { publish: Vec<String>, subscribe:
    Vec<String> }` variant with independent direction allowlists.
  - `starter-ext-supervisor`: `event_bus` namespace gated under
    `EventBus`; both `event_bus.publish` and `event_bus.subscribe`
    accepted when granted; new `event_bus_namespace_gated` test.
  - `starter-ext-sdk`: new `EventBusHandle { publish(topic,
    payload) -> Result<()> }` backed by a private
    `EventBusBackend` trait; `requires!{}` gains an `event_bus`
    arm; `CtxInner::new` widens to 9 args (the new backend slot).
    All 6 adapters (REST, CLI, workers, gRPC, MCP, WASM) provide
    a `StubEventBus` returning `Error::Capability("event_bus
    not wired …")`.
  - `starter-ext-host::validate`: `capability_matches` learns the
    `cap.event_bus` ⇔ `Capability::EventBus` mapping so a
    manifest requiring the capability without a matching grant
    fails at load.
  - Design landed:
    `docs/design/extensions/event-bus.md`.
- Decisions:
  - **Publish-only v1.** Subscribe is meaningful design surface
    in its own right (backpressure, late-subscriber replay, drop
    semantics) and not session-scoped. Shipping the SPI wire
    type for it now (`EventBusSubscribeRequest`) locks the wire
    shape so adding the handle later doesn't break the supervisor.
  - **Independent direction allowlists.** `publish: []` and
    `subscribe: []` are independent neutralised forms — an
    extension might subscribe to host topics it doesn't itself
    publish, or vice versa.
  - **Best-effort fan-out.** Subscriber whose channel is full
    **drops** the message rather than blocking the publisher.
    Encoded in the deferred subscribe handle; row 4 doesn't
    pin down at-most-once vs at-least-once.
  - **Host-side backend deferred to row 5.** Same release as the
    `WarehouseReadBackend` impl — both belong to the host
    dispatch wiring that bridges JSON-RPC namespaces to host
    services. Stubs in every adapter mean extensions can compile
    against the handle today.
- Next: **Phase 2 gate is now reachable** (row 4 + host backend
  for fan-out) once row 5 lands. Two reasonable next moves:
    1. Pull the host backends forward (the `WarehouseReadBackend`
       impl from row 2's design doc + the `EventBusBackend`
       impl). Both bridge to existing host services; lands the
       Phase 1 + Phase 2 gate tests in one session.
    2. Move to row 5 (`DashboardHandle` + `AuthzHandle`).
       Cleaner per the proposal's phase ordering but the gate
       tests slip.
  Per the proposal's wording, the gates close on the host-side
  backend work; the SDK surface (rows 1–4) ships independently.

### 2026-05-27 — row 3 lands: `contributes.warehouse_templates[]` manifest slice

- Rows moved: row 3: ❌ → ✅ shipped (master).
- Changes:
  - `starter-ext-spi`: new `ContributeWarehouseTemplate { name,
    params_schema, tables, sql_file }` plus
    `Contributes::warehouse_templates: Vec<…>`.
  - `starter-ext-host::validate`: reserved-prefix +
    namespace-ownership checks for `warehouse_templates[].name`
    (matching the `nodes[].kind` shape); `capability_matches`
    learns the `cap.warehouse_read` ⇔ `Capability::WarehouseRead`
    mapping so a manifest requiring the capability without a
    matching grant fails at load.
  - `starter-ext-host::warehouse`: new
    `TemplateRegistry::extend_from_record(&ExtensionRecord)` reads
    each contributed entry's `params_schema` (parsed as JSON) and
    optional `sql_file` (stored verbatim) from the record's bundle
    directory and inserts the resulting `TemplateSpec`. Failed /
    no-manifest records are a no-op.
  - Design landed:
    `docs/design/extensions/warehouse-templates-contribution.md`.
- Decisions:
  - **Silent shadowing** for name collisions — matches every
    other contribute slice today. An explicit `override:` flag +
    hard error is a row-3 follow-up, only worth shipping when
    the first collision is reported in practice.
  - **Schema-meaning validation deferred.** The JSON Schema is
    parsed for syntax only at load time; the call-site `params`
    check belongs to the resolver that lands in row 5.
  - **Per-template table allowlist deferred to row 5.** The
    supervisor still gates the `warehouse` namespace as a whole;
    the per-call `template.tables ⊆ grant.tables` cross-check
    lands when the real `WarehouseReadBackend` impl arrives (it
    already needs the registry for spec lookup, so no extra
    plumbing).
  - Loader stays I/O-free outside manifest reading: file I/O for
    contributed templates runs in `extend_from_record`, called
    by the host integration crate after `commit`, not inside
    `Loader::commit` itself.
- Next: **Phase 1 gate.** Rows 1–3 are now ✅. The gate asks
  for a handwritten test extension performing a tenant-scoped
  `ctx.warehouse_read().query(…)` end-to-end. Two paths from
  here, in priority order:
    1. Pick up the gate test — needs the host-side
       `WarehouseReadBackend` impl that consults the
       `TemplateRegistry` and binds `$caller_tenant_id` from
       `ctx.caller()`. This is the row-5 piece pulled forward
       to a stub that handles only the four builtin templates
       (enough to retire the four hard-coded matches in
       `TimescaleAnalyticsBridge`).
    2. Or move to row 4 (`EventBusHandle`) per the original
       phase ordering; the gate test can land after row 5.
  Per the proposal the gate is what unlocks Phase 2; row 4
  technically blocks on Phase 1 closing.

### 2026-05-27 — row 2 lands: `WarehouseReadHandle` + builtin `TemplateRegistry`

- Rows moved: row 2: ❌ → ✅ shipped (master).
- Changes:
  - `starter-ext-spi`: new `warehouse` module with `Row`,
    `TemplateSpec`, `WarehouseReadRequest`; `Capability::WarehouseRead
    { tables: Vec<String> }` variant.
  - `starter-ext-supervisor`: `warehouse` namespace gated under
    `WarehouseRead`; `category_of` recognises the variant; new
    `warehouse_read_namespace_gated` test.
  - `starter-ext-sdk`: new `WarehouseReadHandle` (`query` /
    `count` / `describe`) backed by a private
    `WarehouseReadBackend` trait; `requires!{}` gains a
    `warehouse_read` arm; `CtxInner::new` widens to 8 args (the
    new backend slot). All six adapters (REST, CLI, workers,
    gRPC, MCP, WASM) provide a `StubWarehouseRead` returning
    `Error::Capability("warehouse_read not wired …")` — the
    real backend lands with row 5's host dispatch wiring.
  - `starter-ext-host`: new `warehouse` module with
    `TemplateRegistry`. `TemplateRegistry::builtin()` registers
    the four templates currently hard-coded in `AnalyticsBridge`
    (`meter_kwh_last_24h`, `meter_litres_last_24h`,
    `meter_value_30d_15m`, `meter_value_24h_1m`).
  - `rubix-agent`: `TimescaleAnalyticsBridge` now holds an
    `Arc<TemplateRegistry>` and uses it as the catalog gate
    — unknown templates are refused at the registry boundary
    before any resolver dispatch. Existing `sqlx` resolvers
    unchanged (row-2 cut-over is catalog-only).
  - Design landed: `docs/design/extensions/warehouse-read.md`.
- Decisions:
  - Resolver execution stays in `rubix-agent`. The `host` crate
    is the catalog source of truth (`TemplateRegistry`) but does
    not depend on `sqlx` or the warehouse client — layering
    hygiene from the proposal stays intact.
  - Per-call **table allowlist** enforcement (cross-checking
    `TemplateSpec::tables` against the grant's `tables: […]`) is
    deferred to when row 3 lands. Until external contributions
    are possible, the four builtin templates all touch
    `samples` and the grant pattern is uniform.
  - V1 surface is sync (`Vec<Row>`). The streaming v2 from
    Appendix A is deferred until a real extension's working
    set forces it; v1 stays on the handle when v2 lands.
  - `_meta.caller`-bound `$caller_tenant_id` binding will be
    enforced by the host-side `WarehouseReadBackend` impl when
    it lands in row 5; today the stubs all refuse with
    `Error::Capability` so no extension can call `warehouse.*`
    via process flavour yet.
- Next: pick up row 3
  (`contributes.warehouse_templates[]` manifest slice). The
  registry is now ready to accept contributed specs — row 3
  is the manifest → `Loader::commit` →
  `TemplateRegistry::insert` glue, plus the manifest-validation
  rules (name shape, no shadowing of builtins without an
  explicit `override:` flag).

### 2026-05-27 — row 1 lands: `CallerIdentity` stamping + `ctx.caller()`

- Rows moved: row 1: ❌ → ✅ shipped (master).
- Changes:
  - `starter-ext-spi`: new `identity` module with `CallerIdentity`
    and `FrameMeta`; `JsonRpcRequest` / `JsonRpcNotification`
    gain an optional `_meta` envelope field (additive, omitted
    from the wire when empty).
  - `starter-ext-supervisor`: `SupervisorHandle::call_as(…)` and
    `send_with_caller(…)` stamp `_meta.caller` via the private
    `stamp_caller` helper. Existing `call` / `send` stay as
    system-frame paths.
  - `starter-ext-sdk`: `CtxInner` carries `Option<Arc<CallerIdentity>>`
    with a `with_caller(…)` builder; `requires!{}`-generated
    Ctx newtypes expose `ctx.caller() -> Option<&CallerIdentity>`
    always (no capability gate — identity is kernel-level).
  - Process flavour dispatch loop extracts `_meta.caller` per
    inbound frame and rebuilds the per-call Ctx.
  - Design landed: `docs/design/extensions/caller-identity.md`.
  - Tests cover SPI round-trip, supervisor stamping (incl. overwrite
    of pre-existing meta), and `CtxInner::with_caller`.
- Decisions:
  - `_meta.caller` rides on the envelope, not inside `params`, so
    handler parameter schemas stay unreserved.
  - System frames (`init`, `health`, `shutdown`, internal cron)
    omit `_meta` entirely; SDK reflects as `ctx.caller() == None`.
  - Builtin + wasm flavour wiring is intentionally deferred to
    rows 2/5; the SPI + supervisor stamping is sufficient to
    unblock the row-2 `WarehouseReadHandle` work, which is what
    the lynchpin role requires.
- Next: pick up row 2 (`WarehouseReadHandle` + builtin
  `TemplateRegistry`). The handle's host-side impl will be the
  first consumer of `ctx.caller()` — it refuses any frame whose
  `tenant_id` is `None`.

### 2026-05-27 — scope and progress docs created

- Rows moved: none — all rows start at ❌ not started.
- Decisions: critical path locked from
  [extension-architecture-north-star.md §"Critical path"](../../proposal/extension-architecture-north-star.md);
  EventBus promoted ahead of `DashboardHandle` (was row 6,
  now row 4); `AnalyticsBridge` recorded as the seed for
  `TemplateRegistry`.
- Next: pick up row 1 (`CallerIdentity` stamping). It is the
  lynchpin and unblocks rows 2, 5. The PR should land
  alongside a new `docs/design/extensions/caller-identity.md`
  in the same commit, per
  [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md).
