---
date: 2026-05-28
---

# 2026-05-28 — `rubix.tenant.update`: rename + relocale

Completes the tenant CRUD trinity. With this verb landed the
shape is now:

| Verb | Op | Snapshot |
| --- | --- | --- |
| `rubix.tenant.create` | `Op::Create` | `after` = new `TenantRow` |
| `rubix.tenant.update` | `Op::Update` | `before`/`after` = full `TenantRow` pair |
| `rubix.tenant.delete` | `Op::Delete` | `before` = prior `TenantRow` |

All three route through the single `TenantReversible` registered
this morning; the `apply_inverse` matrix already handles
`Op::Update`, so no Reversible changes were needed for this slice
— only the new verb file, its DTO, and the wire-ins.

## Why this and not the other candidates

- **Operator surface for `changelog_kind_policy`** — still
  blocked. The concurrent admin session's working set
  (`routes/admin/invoke.rs`, `routes/admin/invoke_stream.rs`,
  `boot/auth.rs`, `boot/migrations.rs`, `boot/mod.rs`,
  `main.rs`, `routes/tools.rs`) was hot at session start. Any
  admin-routes edit from this side would collide. Defer until
  those files go cold.
- **PG-backed `TenantStore`** — bigger slice (needs schema
  migration, swap in `boot/`, an integration test against the
  PG container). Worth its own session.
- **`rubix.tenant.update`** — pure rubix-tools + rubix-spi
  surface. Zero overlap with admin session files. Closes the
  "rename is not yet wired" admission that landed in
  `rubix.tenant.create`'s `when_not_to_use` this morning.

## Design choices

### Field-level optionality, request-level requirement

Both `name` and `locale` are `Option<String>`. Field-level
`None` means "leave this field alone"; field-level `Some("")`
is rejected (almost always a wire-shaped bug — same posture as
`rubix.user.tenant.assign`'s blank handling). A request with
**both** fields `None` is rejected as `Error::Invalid` rather
than silently treated as unchanged: "update with nothing" is a
wire-shaped bug too, and conflating it with the legitimate
unchanged case (where the operator did request fields but they
matched) hides the bug.

### Idempotency, when both fields match prior

If every requested field already matches the stored row, the
verb returns `rubix.tenant.unchanged` and `ReversibleTool::change_for`
returns `None`. Mirrors `user.role.set` / `user.prefs.set` /
`user.tenant.assign`. Without this, undo of a no-op call would
silently rewrite a row the caller never actually flipped.

### Uniqueness on rename — verb-level, not store-level

The `TenantStore` trait already has `create` (enforces
uniqueness) and `put` (bypasses uniqueness, used by
snapshot-restore). Adding a third `update` method would have
forced both methods to take a "current id" parameter for the
uniqueness-excluding-self filter. Kept the trait surface at two
write methods and put the uniqueness-excluding-self check in
the verb (via `store.list().any(|r| r.tenant_id != self && r.name == new)`).

PG-backed implementations will enforce the same invariant via a
unique index on `name` and surface the constraint violation as
`Error::Conflict` from `put`. The verb's pre-check then becomes
a fast path — but it stays in place because it produces a
better error message (knows the conflict is a rename, not a
restore race).

### Renaming an id is intentionally unsupported

The id (`tenant_id`) is immutable. The DTO request struct
deliberately does *not* expose an `id` rename field. Renaming
an id would invalidate every per-tenant FK across the system:
`UserRow.tenant_id`, every `Change.resource.tenant`, every
per-tenant warehouse view, every per-tenant `ResourceRef` in
the audit log. The correct shape for "I want a different id" is
`tenant.create` (new) + reassign users + `tenant.delete` (old),
which the operator surface already supports.

### Self-rename guard

Regression test `renaming_to_own_current_name_does_not_self_collide`:
the uniqueness filter excludes the row being updated. Without
the `r.tenant_id != prior.tenant_id` clause, a relocale that
keeps the name (`{name: "Acme", locale: "es"}` against an
existing `("Acme", "en")` row) would false-positive as a
conflict on the unchanged-name + changed-locale path. The test
locks this in.

## Files touched

### New

- `rubix/crates/rubix-spi/src/dto/tenant/update.rs` — DTO +
  descriptor (`TenantUpdateRequest`, `TenantUpdateResponse`,
  `REQUIRED_PERMISSION = "tenants.write"`).
- `rubix/crates/rubix-tools/src/tenant/update.rs` — `TenantUpdateTool`
  + 12 unit tests:
  - `rename_changes_name_and_emits_updated`
  - `relocale_changes_locale_and_leaves_name_alone`
  - `rename_and_relocale_together_apply_both`
  - `rename_to_existing_name_is_rejected_as_conflict`
  - `rename_to_same_name_is_unchanged_and_skips_draft`
  - `renaming_to_own_current_name_does_not_self_collide`
  - `missing_tenant_returns_not_found`
  - `empty_tenant_id_is_rejected`
  - `no_fields_supplied_is_rejected`
  - `empty_name_is_rejected`
  - `untrimmed_locale_is_rejected`
  - `change_for_records_update_with_before_after_snapshots`
- `rubix/docs/sessions/undo/2026-05-28-tenant-update.md` (this doc).

### Modified

- `rubix/crates/rubix-spi/src/dto/tenant/mod.rs` — add `pub mod update;`
- `rubix/crates/rubix-tools/src/tenant/mod.rs` — add `pub mod update;`
- `rubix/crates/rubix-spi/catalogues/en.json` — 2 keys
  (`rubix.tenant.updated`, `rubix.tenant.unchanged`).
- `rubix/crates/rubix-spi/catalogues/es.json` — 2 keys.
- `rubix/crates/rubix-agent/src/registry.rs` — import +
  `wrap_rev(Arc::new(TenantUpdateTool::new(tenant_store.clone())))`
  inserted between `TenantCreateTool` and `TenantDeleteTool`.
- `rubix/crates/rubix-spi/src/dto/tenant/create.rs` — refreshed
  `when_not_to_use` to point at `rubix.tenant.update` for
  rename, dropping the "not yet wired" admission.
- `rubix/docs/design/undo/README.md` — tenant-lifecycle subsection
  updated to mention all three verbs.

### NOT touched

- `rubix/crates/rubix-tools/src/tenant/store.rs` — no new trait
  methods. `put` covers the write path; uniqueness sits in the
  verb. This is deliberate (see "verb-level, not store-level"
  above).
- `rubix/crates/rubix-store-postgres/migrations/changelog_policy/` —
  the `0002_tenant_audit_floor.sql` seed from this morning
  already pins the `tenant` kind to `max_age_days = NULL`;
  update changes share the existing kind.
- `rubix/crates/rubix-agent/src/admin/`, `routes/admin/`,
  `boot/auth.rs`, `tail_listen.rs` — concurrent admin session
  zone, verified untouched via `git diff --stat`.

## Validation

- `cargo test -p rubix-tools --lib` → **230 passed** (was 218
  this morning; +12 from `update`). All earlier tenant /
  user-tenant-assign tests still green.
- `cargo build --workspace` → clean.
- `cargo test -p rubix-agent --test goal_2_user_admin_test
  --test undo_dispatch_test --test admin_registry_test --test
  admin_openapi_projection_test` → unchanged green. Registry +
  OpenAPI projection picked up the new verb without further
  wiring.

## What's next

- **Operator surface for `changelog_kind_policy`** — still the
  highest-leverage next slice; gate on the admin session going
  cold. Verbs to add: `rubix.audit.policy.get` (list rows),
  `rubix.audit.policy.set({kind, max_age_days})` (operator
  override, `ON CONFLICT (resource_kind) DO UPDATE`). Both
  admin-only, both need to route through whatever pattern the
  admin session lands on.
- **PG-backed `TenantStore`** — the in-memory store still loses
  tenants on restart. Either reuse
  `starter-auth-users::PgTenantStore` (cross-crate import; the
  auth side is canonical for login already) or write a
  `PgRubixTenantStore` in `rubix-store-postgres` that mirrors
  the in-memory trait one-to-one. The unique-name invariant
  needs a partial unique index `WHERE deleted_at IS NULL`
  unless we go hard-delete (we currently hard-delete; index is
  just `UNIQUE(name)`).
- **`rubix.team.update` / `rubix.team.delete`** — symmetric to
  what we just shipped for tenant. Team lifecycle is already
  half-wired (`team.create`, `team.assign`); update + delete
  would close team CRUD too.
- **§3.2 / warehouse Reversibles** — still blocked on Phase B.
