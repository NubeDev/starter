# Admin introspection — route registrar, streaming invoke, persistent audit

**Date:** 2026-05-28 (second pass on the introspection slice).
**Branch:** rubix.
**Scope:** rubix-agent only. Sibling work on `undo-cursor` (touches
`starter-changelog-postgres/src/tail_listen.rs`, `starter-undo/`,
and `rubix-agent::registry::undo_built`) is out of scope and was
not modified.

## TL;DR

Three slices landed in one session, in priority order (3) → (2) → (1):

1. **`RouteRegistrar`** (slice 3) — single chokepoint that records
   `RouteEntry { method, path, description, tags, request_schema,
   response_schema }` parallel to the live axum `Router`. Every
   rubix-agent-owned route mounts through it; upstream routers
   come in via `.merge_external(...)`. `GET
   /api/v1/admin/openapi.json` is projected off the catalog after
   the final merge, so the live router and the published OpenAPI
   doc cannot drift. A workspace discipline test fails CI if any
   `.rs` file outside the registrar contains a raw `.route(`.
2. **Streaming invoke** (slice 2) — extracted `ChatFrame` from
   `chat_stream.rs` into a shared `crate::routes::stream_frames`
   module as `StreamFrame`, with a flat `Done` variant that
   carries *both* chat (token + cost) and admin invoke (status +
   latency) optional keys. Migrated `chat_stream.rs` onto it
   (wire shape unchanged — chat frontend still consumes
   `frame.input_tokens` etc.). Added
   `POST /api/v1/admin/registry/tools/{id}/invoke/stream` mounted
   under the same `with_principal + with_role(Admin) +
   with_scope("admin:invoke")` sandwich as the sync sibling,
   emitting `connected → result → done { status: "ok",
   latency_ms }` (or `error` on failure).
3. **Persistent invoke audit** (slice 1) — generalised
   `middleware::ChangelogState`'s `tool_path_prefix: String` to
   `tool_path_prefixes: Vec<String>`. Wrapped the admin sync +
   streaming invoke routers in the same `changelog_layer` as the
   public dispatcher, configured with both
   `/api/v1/tools/` and `/api/v1/admin/registry/tools/`. Each
   successful admin invoke now writes one `tool.invoke` row
   attributed to the admin's subject; the captured payload
   preserves the request's `tenant` field for SIEM join.

## Files touched

### New

- `rubix/crates/rubix-agent/src/routes/registrar.rs` — registrar +
  `RouteEntry` + `catalog_to_openapi` projection.
- `rubix/crates/rubix-agent/src/routes/stream_frames.rs` — shared
  `StreamFrame` + `frame_to_sse`, with `done_chat(...)` /
  `done_invoke(...)` helpers.
- `rubix/crates/rubix-agent/src/routes/admin/openapi.rs` — registrar
  for the projected `/api/v1/admin/openapi.json`.
- `rubix/crates/rubix-agent/src/routes/admin/invoke_stream.rs` —
  SSE streaming invoke handler.
- `rubix/crates/rubix-agent/tests/route_registrar_discipline_test.rs`
- `rubix/crates/rubix-agent/tests/admin_openapi_projection_test.rs`
- `rubix/crates/rubix-agent/tests/admin_invoke_stream_test.rs`
- `rubix/crates/rubix-agent/tests/admin_invoke_audit_test.rs`

### Migrated

- Every admin route module
  (`tools|nodes|rules|templates|tables|skills|extensions|overview|registry|invoke`)
  now exposes `pub(super) fn registrar(state) -> RouteRegistrar`
  with full `RouteMeta` (describe + tag + request_schema).
- `routes/{dashboard_events,flow_events,chat_stream,flow_run,openapi_doc,tools}.rs`
  gained `pub fn registrar(state) -> RouteRegistrar`; the old
  `pub fn router(state) -> axum::Router` survives as a
  `.into_router()` alias so existing tests still compile.
- `routes/admin/mod.rs` exports `admin_registrar`,
  `admin_invoke_registrar`, `admin_invoke_stream_registrar`,
  `admin_openapi_registrar`; backwards-compat `admin_router` /
  `admin_invoke_router` shims kept.
- `health.rs` gained `*_registrar()` builders alongside `*_router()`.
- `main.rs` composes via `RouteRegistrar` end-to-end; the admin
  OpenAPI doc is projected from `app.catalog()` after every
  merge, then mounted as a final registrar.
- `middleware/changelog.rs`: `tool_path_prefix: String` →
  `tool_path_prefixes: Vec<String>`; helper renamed to
  `tool_id_from_any_prefix`; covered by a new prefix-match test.
- `chat_stream.rs`: local `ChatFrame` + `frame_to_sse` dropped,
  call sites switched to the shared `StreamFrame` and
  `done_chat(...)` helper. Wire shape byte-compatible.
- `tests/changelog_middleware_test.rs`: updated the single
  `tool_path_prefix` literal to the new `Vec` field.

### Docs

- `rubix/docs/design/admin/README.md`:
  - Endpoint list now shows `GET /api/v1/admin/openapi.json` and
    `POST /admin/registry/tools/{id}/invoke/stream`.
  - "Invocation" section calls out the persistent audit posture
    (same `changelog_layer`, both path prefixes, `tenant` in
    payload).
  - New "Streaming invoke" subsection documents the frame
    sequence and the unified `Done` variant.
  - "What this surface is not" trimmed: streaming dispatcher and
    OpenAPI-source-of-truth carve-outs removed.
  - New "OpenAPI projection and route registrar discipline"
    section documents the chokepoint + the workspace discipline
    test.

## Tests added / passing

| Test | Count | Notes |
|---|---:|---|
| `route_registrar_discipline_test` | 1 | bans raw `.route(` outside the registrar |
| `admin_openapi_projection_test` | 2 | every mounted path + operationId uniqueness |
| `admin_invoke_test` | 6 | pre-existing; still green after migration |
| `admin_invoke_stream_test` | 5 | new; success + 400 + 404 + 403 + 401 |
| `admin_invoke_audit_test` | 2 | admin invoke writes one `Change`; anonymous writes none |
| `changelog_middleware_test` | 3 | pre-existing; updated for new `Vec` field |
| `admin_registry_test` | 9 | pre-existing; clean |
| `admin_schema_sweep_test` | 1 | pre-existing; clean |
| `stream_frames` unit tests | 4 | inside the lib |

`cargo check -p rubix-agent --lib --bins --tests` clean.

## Known pre-existing failure (out of scope)

`routes::chat_stream::tests::skill_body_for_hint_resolves_bundled_skill`
fails on `HEAD` as well as on this branch — the bundled dashboard
skill body now starts with an `ALWAYS … BANANA` prefix instead of
the expected `# Dashboard builder` heading. This is an unrelated
content-bundle issue; isolated by stashing this session's changes
and re-running the test on raw `HEAD`. Not touched.

## Continuation hooks

- **`Tool::invoke_stream` / sidecar `StreamingTool`.** Today every
  tool implements `Tool::invoke` synchronously; the streaming
  admin handler awaits the sync result and translates it into
  `connected → result → done`. When a long-running tool needs to
  emit progress frames mid-flight, add `invoke_stream` to the
  `Tool` trait (default impl wraps `invoke` exactly the way the
  admin handler does today). The wire shape stays the same.
- **`admin:read` scope.** Currently `Role::Admin` browses
  everything; the read-only split documented in the README is
  still a future task.
- **Persistent audit for the read surface.** Browsing the catalog
  doesn't write a `Change` row — only invokes do. If we want
  read-side audit (e.g. for "who looked at the warehouse table
  list at 03:00"), add a second middleware or extend the
  `changelog_layer` matcher to include read paths with a different
  op tag.
- **Catalog metadata for upstream routers.** The OpenAPI
  projection only covers rubix-agent-owned routes. To project
  starter-* routes too, either move those routers to expose a
  `RouteRegistrar` of their own, or have rubix-agent describe
  them out-of-band via a small `merge_external_with_meta(...)`
  helper.
- **Response schemas.** `RouteMeta` carries `response_schema:
  Option<Value>` but most routes don't fill it in yet. Pass
  through each `routes/admin/*` registrar and add canonical
  envelopes once `schemars` for our DTOs lands.
