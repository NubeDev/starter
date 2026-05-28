# 2026-05-28 — `rubix.user.enable` (user account-state symmetry)

Closes the asymmetry exposed by [team-unassign](./2026-05-28-team-unassign.md): every other goal now has its own re-enable / un-do path, but disabled users only had `rubix.undo.last`. That works for the disabling actor — and fails the moment another operator owns the cleanup. This slice adds the canonical re-enable verb.

## Why a dedicated verb, not "just undo"?

`rubix.undo.last` is **per-actor**: an operator can only undo their own most recent reversible mutation. That posture is correct — cross-actor undo would let any admin walk over any other's history. But it leaves a hole in operator workflows:

1. Admin A disables Ada (offboarding starts).
2. Offboarding is cancelled.
3. Admin B is on shift and needs to re-enable Ada.

Pre-slice, B had to either escalate back to A or hand-edit the `users` table. Post-slice, B runs `rubix.user.enable` — same `users.write` permission as `disable`, same idempotency posture.

The verb does **not** replace undo. A who disabled by mistake can still undo their own action. Enable is the cross-actor surface.

## Design choices

- **Idempotency mirror.** `was_already_enabled` ↔ `was_already_disabled`. `change_for` returns `None` in the no-op case so undo cannot re-disable a user the caller never flipped, and so the redo stack is preserved under §3.4. Diagnostic: `rubix.user.already_enabled`.
- **Echo `prior_disabled_at_ms` on response.** The §3.1 echo rule says every identity-bearing field flows through the response so `change_for` reconstructs the snapshot byte-exact. For enable, the *one* field that changes is `disabled_at_ms` — so the prior timestamp MUST appear on the response. Without it, undo of enable would restore `Some(now())` instead of the original `Some(prior_ts)`, and the `disabled` audit row would lose its original context.
- **Snapshot shape (not patch).** Consistent with the rest of the user kind. `UserReversible::apply_inverse`'s `Op::Update` arm already calls `parse_row` on `before` and treats it as canonical full state. Flipping enable to patch shape alone would force a kind-wide change; the cost is unjustified for a single-field flip in a < 1 KB row.
- **Resolve by id or email.** Mirrors `disable`. `user_id` wins when both are passed.
- **Permission: `users.write`.** Same as disable. Both are account-state lifecycle and they share an authorisation boundary; splitting them would force every "lifecycle operator" role to carry two permissions.
- **No `email` change, no `role` change, no `prefs` change.** Enable is single-purpose. Use `rubix.user.role.set` or `rubix.user.prefs.set` for those.

## Files touched

### New
- `rubix/crates/rubix-spi/src/dto/user/enable.rs` — `UserEnableRequest`, `UserEnableResponse`, `REQUIRED_PERMISSION`, `DESCRIPTOR`.
- `rubix/crates/rubix-tools/src/user/enable.rs` — `UserEnableTool` (Tool + ReversibleTool) + 7 tests.

### Modified
- `rubix/crates/rubix-tools/src/user/store.rs` — `UserAdminStore::enable(user_id) -> Result<(UserRow, UserRow)>` trait method + `InMemoryUserStore` impl. Contract: missing user → `Error::NotFound`; already enabled → `Ok((prior, prior))`.
- `rubix/crates/rubix-tools/src/user/mod.rs` — `pub mod enable;`.
- `rubix/crates/rubix-spi/src/dto/user/mod.rs` — `pub mod enable;`.
- `rubix/crates/rubix-spi/catalogues/en.json` — `rubix.user.enabled` + `rubix.user.already_enabled`.
- `rubix/crates/rubix-spi/catalogues/es.json` — Spanish equivalents.
- `rubix/crates/rubix-agent/src/registry.rs` — wire `UserEnableTool` after `UserDisableTool`.
- `rubix/crates/rubix-spi/src/dto/user/disable.rs` — `when_not_to_use` refreshed: drop the "not yet wired" admission on team.unassign (it shipped earlier today), add a positive pointer to `rubix.user.enable`.

## Tests

`cargo test -p rubix-tools --lib user::` — **39 user-goal tests pass** (was 32, +7).

The seven new tests in `enable.rs`:

1. `enable_disabled_user_emits_enabled_diagnostic` — happy path, prior timestamp echoed.
2. `enable_already_enabled_emits_already_and_skips_draft` — idempotency: diagnostic + `change_for` returns `None`.
3. `second_enable_after_real_one_is_idempotent_and_skips_draft` — same posture across two calls.
4. `missing_user_returns_not_found` — wire-shaped bug surface.
5. `empty_request_is_rejected` — Invalid when neither id nor email present.
6. `change_for_records_update_with_byte_exact_snapshots` — locks in the §3.1 echo contract: `before.disabled_at_ms == Some(100)`, `after.disabled_at_ms == None`, prefs + tenant preserved on both halves.
7. `reversible_round_trip_restores_original_disabled_timestamp` — end-to-end: invoke → draft → assemble Change → `UserReversible::apply_inverse` → live row carries `Some(100)` again, *not* a fresh `now`. This test locks the byte-exact timestamp contract that the echo rule exists for.

## Validation totals

- `cargo build --workspace` — clean.
- `cargo test -p rubix-tools --lib` — **260 passed** (was 253, +7).
- `cargo test -p rubix-agent --test undo_dispatch_test` — 3/3.
- `cargo test -p rubix-agent --test goal_2_user_admin_test` — 9/9.
- `cargo test -p rubix-agent --test admin_registry_test` — 2/2.
- Clippy clean on user::* files.

## Admin-session zone

Untouched: `rubix-agent/src/admin/`, `rubix-agent/src/routes/admin/`, `boot/auth.rs`, `crates/starter-changelog-postgres/src/tail_listen.rs`.

## What's next

The continuous-shipping queue is shrinking. Remaining candidates:

- **PG-backed `UserAdminStore` / `TenantStore` / `TeamAdminStore`.** All three are in-memory today and lose every row on restart. `starter-auth-users::PgTenantStore` already exists for the auth side — could reuse or write a sibling `PgRubixTenantStore` in `rubix-store-postgres`. Picking TenantStore first is the simplest slice (no membership map, no lifecycle flags). User second. Team last (membership map = one extra table + JOIN on read).
- **Operator surface for `changelog_kind_policy`.** `rubix.audit.policy.get` + `rubix.audit.policy.set({kind, max_age_days})` with ON CONFLICT UPDATE. Both admin-only. Still partially blocked on admin-session zone — need to check the wiring once that branch lands.
- **`rubix.user.delete`.** Genuine hard-delete (not disable). High risk because user rows have FKs in audit / prefs / membership. Probably wants a refuse-if-referenced posture mirroring `tenant.delete`. Worth its own dedicated slice with cascade decision recorded up front.
- **§3.2 node-level undo** — still gated on Phase B flow_nodes.
- **Warehouse-write Reversibles** — still deferred.

## Cross-references

- [`./2026-05-28-tenant-lifecycle.md`](./2026-05-28-tenant-lifecycle.md) — tenant CRUD pair (create + delete).
- [`./2026-05-28-tenant-update.md`](./2026-05-28-tenant-update.md) — tenant trinity closeout.
- [`./2026-05-28-team-crud-closeout.md`](./2026-05-28-team-crud-closeout.md) — team update + delete.
- [`./2026-05-28-team-unassign.md`](./2026-05-28-team-unassign.md) — cascade-on-delete footgun closure; surfaced the asymmetry this slice addresses.
- [`../../design/undo/README.md`](../../design/undo/README.md) — proposal §3.1 (echo rule) and §3.4 (no-op preserves redo stack).
