# Extensions — North-Star scope

> **Tier:** scope (plan). Lifetime: weeks–months. Per
> [HOW-TO-CODE.md §0a](../../../HOW-TO-CODE.md) **source code must
> not reference any file in this folder.** When a section lands,
> promote the present-tense parts into `docs/design/extensions/`
> and update code links to point there.

## What this scope is

The plan to deliver the **typed-capability extension architecture**
described in:

- [docs/proposal/extension-architecture-north-star.md](../../proposal/extension-architecture-north-star.md)
  — the three rules, the capability roadmap, Appendix A
  (`WarehouseReadHandle` design sketch), the cross-tenant
  sharing model, and the critical-path ordering.
- [docs/proposal/extension-architecture-diagrams.md](../../proposal/extension-architecture-diagrams.md)
  — simple view, full view, end-to-end request trace.

The proposal is the *target shape*. This scope is the *work
list* that gets us from today's five shipped handles
(`Secrets`, `HttpOut`, `Fs`, `WallClock`, `Tracing`) to a
surface that supports a Power-BI-style dashboard-authoring
extension end-to-end.

## Why this scope exists

Verified against the code as of this draft:

| Area | State today | Source |
|---|---|---|
| Three-flavour SDK (`builtin`, `process`, `wasm`) | ✅ shipping — builtin + process fully wired; wasm awaits first capability call | [`starter-extensions/crates/starter-ext-sdk/`](../../../../starter-extensions/crates/starter-ext-sdk/) |
| JSON-RPC wire format with stream sub-protocol | ✅ shipping | [`starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs`](../../../../starter-extensions/crates/starter-ext-spi/src/jsonrpc.rs) |
| REST → JSON-RPC dispatcher (process flavour) | ✅ shipping at `/api/v1/tools/{id}` | [`starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs`](../../../../starter-extensions/crates/starter-ext-server/src/rest/dispatcher.rs) |
| `requires!{}` macro + per-extension typed `Ctx` | ✅ shipping | [`starter-extensions/crates/starter-ext-sdk/src/lib.rs`](../../../../starter-extensions/crates/starter-ext-sdk/src/lib.rs) |
| Five capability handles | ✅ shipping | [`starter-extensions/crates/starter-ext-sdk/src/ctx.rs`](../../../../starter-extensions/crates/starter-ext-sdk/src/ctx.rs) |
| Module-Federation UI hand-off (`factory.init(handle)`) | ✅ shipping (singletons: react, react-dom, react-query, zustand; hooks: formatters, prefs, intl; ctor: StarterClient) | [`rubix/frontend/src/lib/extension-host.ts`](../../../frontend/src/lib/extension-host.ts) |
| Reference extension layout | ✅ shipping | [`rubix/extensions/com.rubix.example/`](../../../extensions/com.rubix.example/) |
| `AnalyticsBridge` with 4 hard-coded warehouse templates | ✅ shipping (host-internal — seed for `TemplateRegistry`) | [`rubix/crates/rubix-agent/src/sdui/analytics_bridge.rs`](../../../crates/rubix-agent/src/sdui/analytics_bridge.rs) |
| `PgDashboardStore` (dashboards table + history) | ✅ shipping (host-internal — first consumer of `DashboardHandle`) | [`crates/starter-store-postgres/src/dashboards/`](../../../../crates/starter-store-postgres/src/dashboards/) |
| `CallerIdentity` stamping in supervisor | ❌ not implemented | — |
| `WarehouseReadHandle` + `TemplateRegistry` | ❌ not implemented | — |
| `contributes.warehouse_templates[]` manifest slice | ❌ not in `Contributes` struct | [`starter-extensions/crates/starter-ext-spi/src/manifest.rs`](../../../../starter-extensions/crates/starter-ext-spi/src/manifest.rs) |
| `EventBusHandle` | ❌ not implemented | — |
| `DashboardHandle`, `AuthzHandle` w/ cross-tenant grants | ❌ not implemented | — |
| `rest[]`, `sse[]`, `grpc[]`, `workers[]`, `cli[]` contribution slices | ⚠️ manifest schema + SDK adapters compile; not mounted in `rubix-agent` boot composer | [`rubix/crates/rubix-agent/src/main.rs`](../../../crates/rubix-agent/src/main.rs) |

## Goal

Ship enough capability handles + contribution slices that a
third-party extension can build a Power-BI-style dashboard
product end-to-end without (a) breaking any of the three rules,
(b) using the `HttpOutHandle` → host-loopback shortcut, or
(c) requiring a host release per new chart.

## Non-goals

- **Wasm capability backends.** Wasm flavour lags per handle.
  Each PR ships builtin + process; wasm is a separate
  follow-up per row.
- **A new transport.** Every handle rides existing JSON-RPC
  stdio + `stream.event` notifications.
- **A `host_call` escape hatch.** SCOPE R6 stays.
- **Calculated-field free-expression engine.** Closed-grammar
  template parameters cover 90% of the use case; arbitrary
  expressions reopen the audit problem the typed-capability
  rule exists to close.
- **Per-extension TLS / CORS / authz middleware.** The host
  owns the perimeter; no exception.

## Critical path

Promoted from
[extension-architecture-north-star.md §"Critical path"](../../proposal/extension-architecture-north-star.md).
The numbered rows below are the work items. Status lives in
[PROGRESS.md](./PROGRESS.md) and is updated at the end of each
session.

### Phase 1 — MVP gate (dashboard authoring possible)

1. **`CallerIdentity` stamping in supervisor.** Adds a
   `CallerIdentity { tenant_id, user_id, roles, request_id }`
   struct in `starter-ext-spi`; the supervisor stamps it onto
   every inbound JSON-RPC frame; SDK exposes `ctx.caller()`.
   Lynchpin of Rule 3 — nothing tenant-scoped ships before this.
2. **`WarehouseReadHandle` + builtin `TemplateRegistry`.**
   Lift the four `AnalyticsBridge` templates into
   `TemplateRegistry::builtin()`; route `AnalyticsBridge`
   through the registry; add the capability handle. Server-defined
   templates only; streaming `BoxStream<Row>` results;
   `$owning_tenant_id` placeholder bound by the host.
3. **`contributes.warehouse_templates[]`.** Add the field to
   `Contributes` in `starter-ext-spi::manifest`. Templates
   contributed by an installed extension become first-class
   members of the registry, gated by the same operator install
   step that gates `contributes.nodes[]` / `contributes.skills[]`.

### Phase 2 — Keep it usable under load

4. **`EventBusHandle`.** `publish(topic, payload)` / `subscribe(topic) -> Stream`.
   Cross-filter and live updates push instead of poll. Promoted
   ahead of dashboard persistence because cross-filter as N HTTP
   round-trips per click breaks at >10 concurrent viewers, while
   dashboards-via-loopback works today.

### Phase 3 — Killer feature (sharing)

5. **`DashboardHandle` + `AuthzHandle` with cross-tenant grants.**
   Ship together. `DashboardHandle` wraps `PgDashboardStore`;
   `AuthzHandle.can(action, resource)` resolves cross-tenant
   grants and the resulting `owning_tenant_id` flows through to
   `WarehouseReadHandle`. Both handles MUST take `owning_tenant_id`
   as an explicit parameter from day one — retrofitting it later
   means re-versioning every tenant-scoped capability.

### Phase 4 — Polish

6. **`BlobHandle` + `ExportHandle`.** PDF / CSV export.
   Browser-side CSV is fine for v1; PDF needs server-side.
7. **`CronHandle`.** Scheduled refresh.

### Plumbing (independent, can run in parallel)

P1. **Mount `rest[]` / `sse[]` / `workers[]` / `cli[]` adapters
in the `rubix-agent` boot composer.** Manifest schema and SDK
adapters compile today; just not wired. Estimated afternoon, not
quarter.

P2. **`HttpOutHandle` v2 with per-authority path allow-lists.**
Tightens the soft trust boundary the loopback shortcut exposes.
Independent of the dashboard work; track separately.

## How each row ships

Per [HOW-TO-CODE.md](../../../HOW-TO-CODE.md) and
[NEW-SESSION.md](../../../NEW-SESSION.md):

1. The work is scoped to one capability handle (or contribution
   slice) per PR.
2. Crate placement per [FILE-LAYOUT.md](../../../FILE-LAYOUT.md):
   - `starter-ext-spi` — add the `Capability` enum variant + wire
     types.
   - `starter-ext-sdk` — add the `*Handle` type in `ctx.rs` +
     wire into `requires!{}`.
   - `starter-ext-host` — builtin backend.
   - `starter-ext-supervisor` — process backend + wire-gate.
   - `starter-ext-wasm` — wasm backend (separate PR per handle).
3. Layering per
   [docs/design/layering/](../../design/layering/README.md):
   capability handle = transport (extension-side); the wire-gate
   in the supervisor is also transport; the host-side impl that
   each handle calls into is domain.
4. Tests live with the code, same PR.
5. **Promote the relevant subsection of the proposal into
   [docs/design/extensions/](../../design/extensions/) in the
   same PR that lands the handle.** Present-tense, describing
   the handle as it is. Strip the "we will…" framing — that
   lives here in scope.
6. Update [PROGRESS.md](./PROGRESS.md) at the end of the session
   that lands the PR.

## Open questions

Tracked here, not in code:

- Template versioning: same major+minor as capabilities, or a
  separate registry version? (Appendix A open question 1.)
- `count(template, params)` — own template entries, or derive
  from the `query` template by rewriting the projection?
  (Appendix A open question 2.)
- `EXPLAIN`-driven cost-rejection and per-extension fairness
  budgets — defer to a later milestone. (Appendix A open
  question 3.)
- Cross-tenant grant table shape: where does it live in
  `starter-store-postgres`? Resolution gates Phase 3.
- Closed-grammar calculated fields: defer to its own scope note
  once Phase 1 + 2 ship and a real consumer asks for it.

## References

- [docs/proposal/extension-architecture-north-star.md](../../proposal/extension-architecture-north-star.md)
- [docs/proposal/extension-architecture-diagrams.md](../../proposal/extension-architecture-diagrams.md)
- [starter-extensions/DOCS/extensions/scope/SCOPE.md](../../../../starter-extensions/DOCS/extensions/scope/SCOPE.md)
  — R1, R6, R8 (the existing extension contract).
- [docs/design/extensions/README.md](../../design/extensions/README.md)
  — where each shipped row promotes to.
- [PROGRESS.md](./PROGRESS.md) — session-by-session status.
