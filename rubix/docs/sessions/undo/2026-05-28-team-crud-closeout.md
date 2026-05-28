---
date: 2026-05-28
---

# 2026-05-28 — Team CRUD closeout: `rubix.team.update` + `rubix.team.delete`

Symmetric follow-up to this morning's tenant CRUD trinity. Team
lifecycle is now:

| Verb | Op | Snapshot shape |
| --- | --- | --- |
| `rubix.team.create` | `Op::Create` | full `TeamRow` in `after` |
| `rubix.team.update` | `Op::Update` | **patch** with only flipped fields |
| `rubix.team.delete` | `Op::Delete` | full `TeamRow` in `before` (including members) |
| `rubix.team.assign` | `Op::Update` | patch with `members` only |

All four route through the single `TeamReversible` already
registered; the `apply_inverse` matrix handles all three ops
symmetrically.

## Why this slice

- Closes the team CRUD gap so the team surface matches tenant.
- Zero overlap with the still-active admin session
  (`routes/admin/*`, `boot/auth.rs`, etc.).
- The "team.update" rename verb's patch-shape draft is a
  meaningful test of the `TeamReversible::apply_inverse` merge
  logic: a rename patch with `members: None` should leave the
  current row's members untouched. The
  `rename_preserves_membership_via_put` test locks that in.

## Design choices

### Patch shape vs snapshot shape

`TeamReversible` already chose patch shape (since assign mutates
only the `members` field). `update` inherits the contract: only
the fields the verb actually flipped land in the patch. If only
`name` changed, the patch carries `name: Some(...)` and leaves
`description: None` / `members: None`. The
`change_for_emits_patch_with_only_flipped_fields` test verifies
this — undo therefore can't clobber concurrent description or
membership edits.

`delete` uses snapshot shape (`Op::Delete` carries the full row
including members in `before`); this matches how
`TeamReversible::apply_inverse` already handles `Op::Delete`
(via `parse_row`, not `parse_patch`).

### Cascade-on-delete: **allow with disclosure**

Documented in the DTO module doc. The decision matrix here is
different from `tenant.delete`:

| | tenant.delete | team.delete |
| --- | --- | --- |
| Membership shape | external FK on `UserRow.tenant_id` | inline `BTreeMap` inside `TeamRow` |
| Orphan risk on delete | yes — users would point at gone tenant | no — members vanish with the row |
| Operator unassign verb? | yes (`rubix.user.tenant.assign` with `null`) | **no** (`rubix.team.unassign` doesn't exist) |
| Verdict | refuse-if-users-assigned | allow + disclose member count |

Refusing team.delete on member-count > 0 would deadlock the
operator: there is no `team.unassign` verb today, so the only
way to clear members is to delete the team. A refuse with no
escape valve is a footgun.

The disclosure mechanism:
- The diagnostic `rubix.team.deleted` interpolates the member
  count (`{members}`), so the operator sees "Team Ops deleted
  with 3 member(s) cascaded".
- The full membership map rides on the response and lands in
  the audit row's `before` snapshot, so undo restores the team
  byte-exact (members re-attached). Verified by the
  `reversible_round_trip_restores_team_with_members` test.

### Immutable id, "outer Option" semantics for `description`

Same call as `tenant.update`: id is immutable; trying to express
"clear description to NULL" as `Option<Option<String>>` would
require serde gymnastics to distinguish `null` from "absent".
We deliberately kept the type flat — `description: Option<String>`
— and document that an empty string is the "clear" idiom.
`clear_description_via_empty_string` test locks that in.

If we ever need true tri-state clear/leave/set semantics, the
move is `serde_with::rust::double_option` rather than rolling
our own deserializer.

### Store trait grew: `list` added; `delete` made strict

`TeamAdminStore::list` was missing — the trait only exposed
`get` by id. Update needs it for the rename uniqueness check.
Added as a first-class method; the in-memory impl is one line.

`TeamAdminStore::delete` previously returned `Ok(())` silently
on a missing id. Made it return `Error::NotFound` to match
`TenantStore::delete`. Side effect: `TeamReversible` Op::Create
undo paths will now propagate the NotFound if the team was
already gone for some other reason — which is the correct
audit-faithful behaviour (silently succeeding would mask the
inconsistency).

### Self-rename guard

The uniqueness filter excludes the row being updated
(`r.team_id != prior.team_id`). Mirrors the tenant test.
Implicitly covered by the `unchanged_path_skips_draft` test:
a rename-to-same-name doesn't collide, doesn't write, and skips
the draft.

## Files touched

### New

- `rubix/crates/rubix-spi/src/dto/team/update.rs` — DTO +
  descriptor.
- `rubix/crates/rubix-spi/src/dto/team/delete.rs` — DTO +
  descriptor (cascade rationale in module doc).
- `rubix/crates/rubix-tools/src/team/update.rs` — verb + 10
  unit tests:
  - `rename_changes_name_and_emits_updated`
  - `re_describe_changes_description_only`
  - `clear_description_via_empty_string`
  - `rename_to_existing_name_is_rejected_as_conflict`
  - `unchanged_path_skips_draft`
  - `missing_team_returns_not_found`
  - `no_fields_supplied_is_rejected`
  - `empty_name_is_rejected`
  - `change_for_emits_patch_with_only_flipped_fields`
  - `rename_preserves_membership_via_put`
- `rubix/crates/rubix-tools/src/team/delete.rs` — verb + 5
  unit tests including the reversible round-trip
  (`reversible_round_trip_restores_team_with_members`).
- `rubix/docs/sessions/undo/2026-05-28-team-crud-closeout.md`
  (this doc).

### Modified

- `rubix/crates/rubix-spi/src/dto/team/mod.rs` — `pub mod delete;`
  + `pub mod update;`.
- `rubix/crates/rubix-tools/src/team/mod.rs` — same.
- `rubix/crates/rubix-spi/catalogues/en.json` — 3 keys
  (`rubix.team.updated`, `rubix.team.unchanged`,
  `rubix.team.deleted`).
- `rubix/crates/rubix-spi/catalogues/es.json` — 3 keys.
- `rubix/crates/rubix-tools/src/team/store.rs` —
  `TeamAdminStore::list` added; `TeamAdminStore::delete` made
  strict (returns `Error::NotFound`).
- `rubix/crates/rubix-agent/src/registry.rs` — imports +
  `wrap_rev(Arc::new(TeamUpdateTool::new(...)))` /
  `wrap_rev(Arc::new(TeamDeleteTool::new(...)))` inserted
  between `TeamCreateTool` and `TeamAssignTool`.
- `rubix/crates/rubix-spi/src/dto/team/create.rs` —
  `when_not_to_use` refreshed to point at the new verbs.

### NOT touched

- `TeamReversible` — patch shape and snapshot-on-delete already
  matched what the new verbs needed.
- `rubix-store-postgres/migrations/changelog_policy/` — `team`
  kind was pinned to the audit floor in the morning's
  `0001_audit_floor_seed.sql`.
- Admin-session zone (`rubix-agent/src/admin/`,
  `routes/admin/`, `boot/auth.rs`, `tail_listen.rs`) — verified
  untouched.

## Validation

- `cargo test -p rubix-tools --lib` → **246 passed** (was 230
  this morning after `tenant.update`; +16 from `team.update` /
  `team.delete` + 1 store test guard).
- `cargo build --workspace` → clean.
- `cargo test -p rubix-agent --test goal_2_user_admin_test
  --test undo_dispatch_test --test admin_registry_test --test
  admin_openapi_projection_test` → unchanged green.

## What's next

- **Operator surface for `changelog_kind_policy`** — still
  blocked on admin session. Highest-leverage next slice once
  unblocked: `rubix.audit.policy.get` + `rubix.audit.policy.set`.
- **`rubix.team.unassign`** — now that team has full CRUD, the
  missing membership operation is unassign. Would shrink the
  cascade-on-delete surface (operator could empty a team before
  deleting). Pattern: copy `rubix.team.assign`, flip the
  membership map mutation, patch-shape draft.
- **PG-backed `TeamAdminStore` / `TenantStore`** — both
  in-memory today; both lose data on restart. Probably one
  slice each, gated on whether `starter-auth-users` already
  has equivalents we can reuse.
- **§3.2 / warehouse Reversibles** — still blocked on Phase B.
