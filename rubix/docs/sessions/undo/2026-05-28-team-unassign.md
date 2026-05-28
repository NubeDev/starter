---
date: 2026-05-28
---

# 2026-05-28 — `rubix.team.unassign`: closes the cascade-on-delete footgun

Closes the explicit "what's next" item from
[`2026-05-28-team-crud-closeout.md`](2026-05-28-team-crud-closeout.md):
the team membership surface was missing an unassign verb, and
that absence was the *reason* `team.delete` had to pick
**allow-with-disclosure** over **refuse-if-members** (no escape
valve to drain a team before deleting it).

With this slice landed:

| Verb | Op | Snapshot |
| --- | --- | --- |
| `team.assign`  | `Op::Update` | patch with `members` only |
| `team.unassign` | `Op::Update` | patch with `members` only (mirror) |

`team.delete`'s cascade posture stays where it landed (allow with
disclosure) — but now it's a real choice rather than a forced
one, because operators can drain a team before deleting it if
they prefer.

## Design choices

### Symmetry with `team.assign`

The verb is a structural mirror of assign: same DTO shape, same
out-of-band `_prior_members` / `_new_members` stash on the JSON
for `change_for` to read, same patch-shape `ChangeDraft`. Kept
the symmetry deliberately so future readers see them as a pair.

### Idempotency: `already_not_member`

Unassign of a non-member is a no-op — returns the
`rubix.team.unassigned` diagnostic with `already_not_member =
true` and produces **no** `ChangeDraft`. Same posture as
`team.assign`'s `already_member` flag. Without this idempotency,
undo of a no-op call would silently rewrite the membership map
to its current state (semantically null, but a wasted audit row
and a redo-stack-clearing side effect under §3.4).

### Missing **team** is NotFound; missing **member** is a no-op

The trait's contract:

- `unassign("t-ghost", "u-1")` → `Error::NotFound { what:
  "team:t-ghost" }`. A non-existent team is a wire-shaped bug;
  silent success would mask it.
- `unassign("t-1", "u-ghost")` where `t-1` exists but has no
  `u-ghost` member → `Ok((prior, prior))`, surfaced as
  `already_not_member = true`. The operator's *intent* (this
  user should not be a member) is satisfied; surfacing it as
  an error would just trigger a retry loop.

This matches how `tenant.delete` treats missing tenants (hard
error) but how `user.tenant.assign` treats unchanged target
tenants (no-op).

### Patch-shape proves its value

The verb gets a dedicated test, `undo_preserves_concurrent_rename`,
which exercises the exact scenario `TeamReversible` was
patch-shaped for:

1. unassign records a patch with `members` only
2. a concurrent rename of the team lands (`name: Ops → Operations`)
3. undo of the unassign replays the patch
4. result: member back, **rename preserved**

If the verb had recorded a snapshot (full row) instead, undo
would have clobbered the rename back to `Ops`. The test locks
in the contract for the future.

### Reversible round-trip preserves the original timestamp

`reversible_round_trip_restores_member_with_original_timestamp`:
the membership map carries `(user_id, assigned_at_ms)` pairs.
Undo restores both halves byte-exact — a re-assign after undo
would have used `now()` for the timestamp, which would be
audit-faithful but not snapshot-faithful. The patch carries the
original `assigned_at_ms` so undo is a true reversal.

## Files touched

### New

- `rubix/crates/rubix-spi/src/dto/team/unassign.rs` — DTO +
  descriptor.
- `rubix/crates/rubix-tools/src/team/unassign.rs` — verb + 7
  unit tests:
  - `unassign_removes_member_and_records_patch_draft`
  - `unassign_non_member_is_idempotent_and_skips_draft`
  - `second_unassign_is_idempotent_and_skips_draft`
  - `missing_team_returns_not_found`
  - `empty_ids_are_rejected`
  - `reversible_round_trip_restores_member_with_original_timestamp`
  - `undo_preserves_concurrent_rename`
- `rubix/docs/sessions/undo/2026-05-28-team-unassign.md` (this doc).

### Modified

- `rubix/crates/rubix-spi/src/dto/team/mod.rs` — `pub mod unassign;`.
- `rubix/crates/rubix-tools/src/team/mod.rs` — `pub mod unassign;`.
- `rubix/crates/rubix-tools/src/team/store.rs` — `TeamAdminStore::unassign`
  added with the missing-team-vs-missing-member contract
  documented inline; `InMemoryTeamStore` impl is one screen
  taller than `assign` because it mirrors the same shape.
- `rubix/crates/rubix-spi/catalogues/en.json` — 1 key
  (`rubix.team.unassigned`).
- `rubix/crates/rubix-spi/catalogues/es.json` — 1 key.
- `rubix/crates/rubix-agent/src/registry.rs` — import +
  `wrap_rev(Arc::new(TeamUnassignTool::new(team_store.clone())))`
  appended right after `TeamAssignTool`.
- `rubix/crates/rubix-spi/src/dto/team/assign.rs` —
  `when_not_to_use` refreshed to drop the "not yet wired"
  admission.

### NOT touched

- `TeamReversible` — patch-shape `apply_inverse`/`apply_forward`
  already handle the mirrored patch. No Reversible changes.
- Admin-session zone — verified untouched.

## Validation

- `cargo test -p rubix-tools --lib` → **253 passed** (was 246
  after team CRUD closeout; +7 from unassign).
- `cargo build --workspace` → clean.
- Agent integration tests (goal_2 / undo_dispatch / admin_registry
  / admin_openapi_projection) → unchanged green.

## What's next

- **Operator surface for `changelog_kind_policy`** — still
  blocked on the admin session. Highest-leverage next slice
  once unblocked.
- **PG-backed `TeamAdminStore` / `TenantStore`** — still in
  memory; both lose data on restart. Probably one slice each.
- **Revisit `team.delete` cascade posture** — now that unassign
  exists, refuse-if-members is a debatable option again.
  Probably still keep allow-with-disclosure (consistency with
  "operators are trusted to read the diagnostic"), but the
  decision is no longer forced.
- **§3.2 / warehouse Reversibles** — still blocked on Phase B.
