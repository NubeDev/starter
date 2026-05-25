# Dashboard tool bodies

## What this design covers

The seven `rubix.dashboard.*` verbs — request / response DTOs,
storage interactions, authz gates, MessageKey catalogue entries.
The verb files exist as stubs today
([`rubix/crates/rubix-tools/src/dashboard/`](../../../crates/rubix-tools/src/dashboard/));
this scope fills them in.

## The seven verbs

| Tool id | Purpose | Reversible? |
|---|---|---|
| `rubix.dashboard.create` | Create a new page (or duplicate a bundled one as a starting point). | Yes — `delete` reverses. |
| `rubix.dashboard.get` | Fetch the active body of a `page_ref` (no caching headers; resolver caches). | n/a |
| `rubix.dashboard.update` | Insert a new revision with optimistic concurrency. | Yes — `undo.last` re-supersedes back to prior. |
| `rubix.dashboard.delete` | Supersede every revision of a `page_id`. Refused for `created_by="system"`. | Yes — `undo.last` re-inserts the previously active revision. |
| `rubix.dashboard.list` | Active pages in the caller's tenant, filtered by tags / owner / search. | n/a |
| `rubix.dashboard.duplicate` | Snapshot an existing page into a new `page_id`. | Yes — `delete` of the duplicate. |
| `rubix.dashboard.history` | Every revision for a `page_id`, newest first. | n/a |
| `rubix.dashboard.page_set` | Atomic write of a full ComponentTree — same as update, but bypasses optimistic concurrency. **AI builder uses this.** | Yes — `undo.last`. |

(`page_set` is the AI-builder's tool of choice because the LLM
won't have seen the latest `expected_revision_id`.)

## Directory layout

```
rubix/crates/rubix-tools/src/dashboard/
  mod.rs              ← barrel + ToolDescriptor entries
  create.rs           ← validate + insert_revision + register authz
  get.rs              ← SELECT active row
  update.rs           ← optimistic-concurrency update
  delete.rs           ← supersede all + check bundled
  list.rs             ← tenant + tags + owner filter
  duplicate.rs        ← clone-into-new-page-id
  history.rs          ← SELECT all revisions
  page_set.rs         ← write atomic body (LLM entry point)
  validate.rs         ← shared ComponentTree validation
  assistant.rs        ← already exists; remains the AI entry point summary (see 06-ai-builder.md)
```

Wire shapes (DTOs) follow per-verb files under
`rubix/crates/rubix-spi/src/dto/dashboard/` — same mirror pattern
as `dto/flow_ops/`.

## Per-verb sketch

### `create.rs`

```text
1. Auth: principal must hold rubix.dashboard:create on the tenant.
2. Validate request:
   - page_id matches /^[a-z][a-z0-9-]{0,62}\.[a-z0-9-.]{1,128}$/
   - body_json deserialises as a valid ComponentTree
   - ir_version == IR_VERSION (refuse forward-compat trees)
   - assign_synthetic_ids before bytes-cap check (R8)
3. store.insert_revision { page_id, body_json, tenant_id, owner_principal, ... }
4. authz.register_resource ResourceSpec { kind, id, actions }
5. Emit MessageKey rubix.dashboard.created (en + es catalogues).
6. Reversible::register — undo recovers by calling delete with the new page_id.
```

≤ 120 LOC. The five steps map to five sub-functions in the same
file (separate helpers in `validate.rs` only if shared with `update.rs`).

### `update.rs`

```text
1. Auth: rubix.dashboard:edit on the page resource.
2. Refuse if created_by="system".
3. Optional expected_revision_id: if set, must match latest active.
   On mismatch → rubix.dashboard.update.conflict (HTTP 409, structured).
4. Validate body (same as create).
5. store.insert_revision (this supersedes prior).
6. Emit rubix.dashboard.updated.
7. Reversible: undo re-inserts the prior body.
```

≤ 140 LOC.

### `delete.rs`

```text
1. Auth: rubix.dashboard:delete on the page resource.
2. Refuse bundled pages with rubix.dashboard.delete_refused_bundled.
3. store.supersede(page_id).
4. authz.unregister_resource.
5. Reversible: undo re-inserts the previously active row.
```

≤ 80 LOC. Note: `delete` only supersedes; the history table keeps
every revision so undo and audit work.

### `page_set.rs`

```text
1. Auth: rubix.dashboard:edit on the page resource (create-if-missing).
2. Validate body.
3. If page exists → store.insert_revision (no optimistic check).
   Else → store.insert_revision as new row + authz.register_resource.
4. Emit rubix.dashboard.page_set.
5. Reversible.
```

This is the verb the AI builder calls. ≤ 100 LOC.

### `list.rs`

```text
1. Auth: rubix.dashboard:list on the tenant.
2. ListFilter { tags[], owner_principal?, search?, cursor?, limit }.
3. store.list_active(caller.tenant, filter).
4. Hide pages without rubix.dashboard:view grant (per-row authz check).
5. Return DashboardSummary[] { page_id, title, tags, owner, updated_at }.
```

≤ 100 LOC.

### `get.rs`, `history.rs`, `duplicate.rs`

Each ≤ 80 LOC. `duplicate` is `get` + `page_set` to a new `page_id`
with `created_by` reset to the caller; bundled-page tag is dropped
so the copy is operator-owned.

## MessageKey catalogue

New keys (every one needs both `en` and `es`):

```
rubix.dashboard.created                       "Dashboard {title} created."
rubix.dashboard.updated                       "Dashboard {title} updated."
rubix.dashboard.deleted                       "Dashboard {title} deleted."
rubix.dashboard.delete_refused_bundled        "Bundled dashboards cannot be deleted."
rubix.dashboard.update.conflict               "Dashboard was modified by someone else. Reload."
rubix.dashboard.page_set                      "Dashboard {title} saved."
rubix.dashboard.duplicated                    "Dashboard {title} duplicated to {new_id}."
rubix.dashboard.not_found                     "Dashboard {page_id} not found."
rubix.dashboard.validation_failed             "Dashboard body failed validation: {reason}."
```

Same commit ships the `es` translations.

## Authz integration

Every tool wraps in the existing three-layer sandwich from
[`docs/design/agent/README.md`](../../design/agent/README.md):

1. `middleware::changelog_layer` — one `starter_changes` row per call.
2. `starter_server::auth::with_principal` — extracts Principal.
3. `starter_authz::with_permission_owned("rubix.dashboard", "<action>")`.

No new middleware. The per-page resource grants come from
`authz.register_resource` (called in `create` / `page_set`) and
the engine consults the cached policy per-call.

## MCP surface

Every dashboard tool auto-surfaces via `FlowAsTool::from_registry`
(R7). No per-tool MCP wiring. The `tools/list` MCP response
includes them after they're registered in the boot tool registry.

## Tests in the same diff

`rubix/crates/rubix-agent/tests/goal_1_dashboards_test.rs`:

1. **End-to-end CRUD** — `create` → `get` → `update` (conflict on
   stale revision_id) → `list` → `history` (returns 2 rows) →
   `delete` → `get` returns None.
2. **Bundled refusal** — `delete` of a `system`-owned page returns
   `rubix.dashboard.delete_refused_bundled`.
3. **Undo round-trip** — `update` then `rubix.undo.last` restores
   the prior body byte-for-byte.
4. **Authz isolation** — two principals in different tenants;
   `list` returns disjoint sets.
5. **Optimistic concurrency** — two concurrent updates; second
   one with stale `expected_revision_id` returns HTTP 409.

## DTOs (mirror `dto/flow_ops/`)

```
rubix/crates/rubix-spi/src/dto/dashboard/
  mod.rs
  create.rs       ← CreateDashboardRequest, CreateDashboardResponse
  get.rs          ← GetDashboardRequest, GetDashboardResponse { body_json, revision_id, ... }
  update.rs       ← UpdateDashboardRequest { expected_revision_id?, body_json }, UpdateDashboardResponse
  delete.rs       ← DeleteDashboardRequest, DeleteDashboardResponse
  list.rs         ← ListDashboardsRequest { tags?, owner?, search?, cursor?, limit }, ListDashboardsResponse
  duplicate.rs    ← DuplicateDashboardRequest, DuplicateDashboardResponse
  history.rs      ← DashboardHistoryRequest, DashboardHistoryResponse { revisions: Vec<DashboardRevisionSummary> }
  page_set.rs     ← PageSetRequest { page_id, body_json }, PageSetResponse
  shared.rs       ← DashboardSummary, DashboardRevisionSummary, ListFilter
```

The OpenAPI doc regenerates from these on the existing
`cargo run --bin gen-openapi` path, and `pnpm codegen` produces
the matching Zod schemas in `@nube/starter-client-ts` (R7).

## Acceptance

1. All seven verbs return 200 against `/api/v1/tools/<id>` with
   valid bodies; 401 / 403 / 404 / 409 cases match the keys above.
2. `goal_1_dashboards_test.rs` passes.
3. The MCP `tools/list` includes `rubix.dashboard.*` after
   boot — verified by the existing MCP integration test scaffold.
4. Each verb file ≤ 200 LOC, validation helper ≤ 200 LOC.
