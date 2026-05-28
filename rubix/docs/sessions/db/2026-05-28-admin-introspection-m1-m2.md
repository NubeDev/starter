# 2026-05-28 — Admin introspection: M1 + M2 invoke landed

Status: M1 shipped end-to-end; M2 synchronous invoke landed with role + scope gate; schema CI gate green.
Branch: `master` (continuous push)
Proposal: [admin-introspection-and-test-console.md](../../proposal/admin-introspection-and-test-console.md)
Design doc: [docs/design/admin/README.md](../../design/admin/README.md)

## TL;DR

Two sessions, one branch. Session 1 shipped the read-only admin
introspection surface (M1). Session 2 added the synchronous invoke
endpoint with an explicit `admin:invoke` scope gate and a schema
sanity sweep that fails CI if a new tool ships without a usable
input schema. Sixteen tests across three integration files pass;
the workspace builds clean apart from one upstream file the
parallel undo-cursor session left mid-refactor (see "Known
issues").

## What landed (this branch, both sessions)

### Wire DTOs — [rubix/crates/rubix-spi/src/dto/admin/](../../../crates/rubix-spi/src/dto/admin/)

- [kind.rs](../../../crates/rubix-spi/src/dto/admin/kind.rs) —
  `RegistryKind` (lowercase singular serde: `tool`, `node`, `rule`,
  `template`, `table`, `skill`, `extension`) + `UnknownKind` error.
- [source.rs](../../../crates/rubix-spi/src/dto/admin/source.rs) —
  `ItemSource` tagged union (`builtin` / `starter` / `extension:<id>`).
- [item.rs](../../../crates/rubix-spi/src/dto/admin/item.rs) —
  `RegistryItem { id, label, summary, source, input_schema,
  output_schema, metadata }` MCP-aligned envelope.
- [snapshot.rs](../../../crates/rubix-spi/src/dto/admin/snapshot.rs)
  / [overview.rs](../../../crates/rubix-spi/src/dto/admin/overview.rs).

### Projection layer — [rubix/crates/rubix-agent/src/admin/](../../../crates/rubix-agent/src/admin/)

- `AdminState` — `Arc`-backed handle bundle (tools map, node
  behaviors, optional rules / templates / extensions).
- `paginate(items, cursor, limit)` — opaque base64 cursor (URL-safe
  no-pad of last item id); default 50, max 200.
- Per-kind projectors: `tools.rs`, `nodes.rs`, `rules.rs`,
  `templates.rs`, `tables.rs`, `skills.rs`, `extensions.rs`,
  `overview.rs`. Each walks the in-memory registry and emits a
  `RegistryItem`.

### Routes — [rubix/crates/rubix-agent/src/routes/admin/](../../../crates/rubix-agent/src/routes/admin/)

- `registry.rs` — `admin_router(state)` builds the read surface:
  `GET /api/v1/admin/registry` (multiplexed snapshot) + the seven
  per-kind sugar routes + `/overview`.
- **`invoke.rs` (M2 — new this session)** —
  `admin_invoke_router(state)` builds `POST /api/v1/admin/registry/tools/{id}/invoke`.
  Body `{ tenant, input }`; tenant required and non-empty; scopes
  a `CallerIdentity` + `actor_local` for the dispatch; maps tool
  `Result<Value, Error>` to HTTP status the same way
  [tools.rs](../../../crates/rubix-agent/src/routes/tools.rs) does;
  emits a `tracing::info!(target: "rubix.admin.invoke", ...)`
  line per call (actor, tenant, tool_id, status, latency_ms).
- `query.rs` / `errors.rs` — shared list-query decoder, 400/404
  shapers.

### Main wiring — [rubix/crates/rubix-agent/src/main.rs](../../../crates/rubix-agent/src/main.rs)

- Builds `AdminState` once at boot from the same handles the
  rest of the agent already owns (no second source of truth).
- Mounts the two routers separately under one `with_principal`
  layer:
  - Read: `with_role(admin_router, Role::Admin)`.
  - Invoke: `with_scope(with_role(admin_invoke_router, Admin), Scope::new("admin:invoke"))`.
- In the no-DB dev path both routers mount ungated (same posture
  as the unguarded tools router on that path).

### Schema CI gate — [tests/admin_schema_sweep_test.rs](../../../crates/rubix-agent/tests/admin_schema_sweep_test.rs)

Workspace test that asserts every `Tool::definition().input_schema`
is a JSON object (empty is allowed for parameterless tools, but
`null` and non-object values fail the build). Currently all
shipped tools pass — the gate stops regressions before merge,
matching the proposal's "CI gate, not runtime fallback" posture.

### Tests

- [admin_registry_test.rs](../../../crates/rubix-agent/tests/admin_registry_test.rs)
  — 9 tests: overview counts, envelope shape, kinds filter,
  unknown kind 400, missing id 404, role gate (401/403/200).
- [admin_invoke_test.rs](../../../crates/rubix-agent/tests/admin_invoke_test.rs)
  — 6 tests: 200 on well-formed body, 400 on missing/blank
  tenant, 404 on unknown tool, 401 on missing principal, 403 on
  admin without `admin:invoke` scope.
- [admin_schema_sweep_test.rs](../../../crates/rubix-agent/tests/admin_schema_sweep_test.rs)
  — 1 gate.

All 16 admin tests pass.

## Session 1 — M1 read-only catalog

The first session covered:
- Reading the proposal + HOW-TO-CODE + FILE-LAYOUT.
- Mapping the codebase via parallel Explore subagents.
- Drafting the design doc.
- Implementing DTOs → projection → routes → main wiring.
- Writing the 9-test integration suite.
- Verifying `cargo build --workspace` clean and `cargo test -p rubix-agent --test admin_registry_test` green.

Decision recorded then: do **not** change `build_tool_registry`'s
signature because 10+ callers (including
[`bin/rubix_admin`](../../../crates/rubix-agent/src/bin/) and every
test) live downstream of it. Instead, the admin path rebuilds a
cheap `RuleRegistry` via
[`build_registry_with_contributions`](../../../crates/rubix-tools/src/cleaner/adapter.rs)
when it needs one.

## Session 2 — M2 invoke + schema gate

This session covered:
- `routes/admin/invoke.rs` — the synchronous JSON invoke handler.
- `routes/admin/mod.rs` — exporting `admin_invoke_router`.
- `main.rs` — splitting the gated mount into read + invoke with
  the `admin:invoke` scope check.
- `tests/admin_invoke_test.rs` — 6 end-to-end tests covering
  happy path, tenant validation, 404, and the scope gate.
- `tests/admin_schema_sweep_test.rs` — the M1 schema CI gate the
  proposal lists as a forcing function.
- Design doc updates: new "Invocation" subsection, new Roles
  table reflecting the role+scope split, updated "What this
  surface is not".

## What's deferred (M2 remainder + M3)

- **SSE streaming invoke.** The proposal's invoke endpoint
  ideally streams `ChatFrame`-shaped frames. Today we ship the
  simpler JSON-in/JSON-out shape — the same shape the existing
  `/api/v1/tools/{id}` router serves. Extracting
  [`ChatFrame`](../../../crates/rubix-agent/src/routes/chat_stream.rs)
  into a shared module + threading a streaming `Tool` variant
  through the dispatch path is its own piece of work.
- **`POST /admin/registry/templates/{name}/query`.** Blocked on
  the RLS-scoped DB role question raised in the proposal's "Open
  questions". Today the agent dispatches every warehouse read as
  the agent's own DB role; until per-tenant DB roles exist, this
  endpoint cannot satisfy the proposal's security posture
  ("inherits RLS").
- **Persistent audit.** The invoke handler emits a structured
  tracing line; wrapping the invoke router in
  [`middleware::changelog_layer`](../../../crates/rubix-agent/src/middleware.rs)
  would persist each call as a `Change` row. Out of scope for
  this session because the existing `changelog_layer` is keyed
  off the `/api/v1/tools/` path prefix; making it admin-aware is
  a small refactor.
- **OpenAPI projection.** Listed in the proposal as M1 day-one
  output of the (not-yet-built) `RouteRegistrar`. Today the
  admin routes are not in the `RouteRegistrar` pipeline because
  the registrar itself does not exist. M3.
- **`RouteRegistrar`.** Proposal calls for one. Every existing
  route would migrate to it; the admin routes are designed so
  they slot in without re-mounting. M3.
- **`HostSlotRegistry` + `/admin/routes` + `/admin/slots`.** M3.
- **Frontend `<ToolTester />`.** M2 deliverable per the proposal;
  not started.

## Known issues outside this work

- [crates/starter-changelog-postgres/src/tail_listen.rs](../../../../crates/starter-changelog-postgres/src/tail_listen.rs)
  is currently broken on master from a parallel undo-cursor
  refactor (missing `Weak` / `tokio::sync::OnceCell` imports,
  `start_shared_listener` arity mismatch). The workspace build
  fails until that lands. Admin tests pass because the failure
  is downstream of `rubix-agent`'s test target compilation path
  by the time the test runner actually links. **Do not patch
  this file from this branch** — the parallel session owns it.

## Files touched this session

```
NEW  rubix/crates/rubix-agent/src/routes/admin/invoke.rs
NEW  rubix/crates/rubix-agent/tests/admin_invoke_test.rs
NEW  rubix/crates/rubix-agent/tests/admin_schema_sweep_test.rs
NEW  rubix/docs/sessions/db/2026-05-28-admin-introspection-m1-m2.md
EDIT rubix/crates/rubix-agent/src/routes/admin/mod.rs
EDIT rubix/crates/rubix-agent/src/main.rs
EDIT rubix/docs/design/admin/README.md
```

## Continuation hooks for the next session

Pick one of:
1. **Persistent invoke audit** — generalise
   [`middleware::changelog_layer`](../../../crates/rubix-agent/src/middleware.rs)
   so it can accept multiple `tool_path_prefix`es (or a path
   matcher closure), then wrap both the tools router and the
   admin invoke router. Adds a test that a successful admin
   invoke produces a `Change` row attributed to the admin
   actor.
2. **Streaming invoke** — extract `ChatFrame` from
   `chat_stream.rs` into a shared module, add a streaming
   variant to the `Tool` trait (or a sidecar `StreamingTool`),
   migrate `chat_stream.rs` + the admin invoke to share one
   frame decoder. Frontend `<ToolTester />` slots in on this.
3. **`RouteRegistrar`** — land the registrar, migrate every
   existing route, project `/api/v1/admin/openapi.json` off
   the catalog. Largest unit of work; unlocks the M3 routes /
   slots surface and removes the duplication between the
   `utoipa::path` macros and the live router.

Recommend (3) → (2) → (1). The registrar removes the need to
hand-wire OpenAPI later; streaming invoke depends only on the
frame extraction (registrar-independent); persistent audit is the
smallest follow-up and reads as a polish step.
