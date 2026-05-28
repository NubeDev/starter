# 2026-05-28 — `rubix.audit.policy.list` + `rubix.audit.policy.set` (operator surface for `changelog_kind_policy`)

Closes the "policy is configurable only by SQL seed" gap that has held since `changelog_kind_policy` was provisioned by `crates/starter-changelog-postgres/migrations/0004_changelog_kind_policy.sql`. Operators now have a tool surface for inspecting and mutating per-kind retention, with full reversibility and idempotency.

## Why now

Two earlier slices closed the major lifecycle gaps in the rubix admin surface (tenant CRUD, team CRUD + assign/unassign, user enable). The pattern that emerged: every operator-facing knob needs a verb, not a SQL migration. `changelog_kind_policy` was the last conspicuous example of "policy lives in SQL seeds" — the audit-floor pins for `user`/`team`/`tenant` are written once at boot via `rubix-store-postgres::changelog_policy/000*_seed.sql` and never adjusted.

Now: an operator who wants to set `flow_def` retention to 90 days, or pin `audit_policy` itself to forever (yes — the policy table is audited too), runs `rubix.audit.policy.set` and gets a recorded, reversible mutation.

## Design choices

- **New goal `audit` (DTO + tool barrel).** Sized for the policy surface specifically — `audit.policy.list`, `audit.policy.set`, room for `audit.policy.clear` (delete a row entirely) if operator demand emerges. Not co-located under `system` because system tools today are operational (`flow_errors`, `alert`) rather than policy. Not under `tenant` because the policy applies across tenants.

- **Snapshot shape Reversible.** Row is tiny `(resource_kind, max_age_days, updated_at)` and the lifecycle includes create/update — mirrors the rationale for `TenantReversible` / `UserReversible` / `TeamReversible`. `Op::Delete` is supported but not currently emitted by any verb.

- **`max_age_days` is tri-state on the wire.** `None` = pinned to forever. `Some(n>0)` = finite retention curve. `Some(n<=0)` is rejected by the verb (the DB schema allows it but it's nonsense — zero would delete every new row instantly). Implicit unbounded (no row) is represented by row absence, not by a sentinel value.

- **`prior` field on response is itself tri-state.** Field absent → no row existed (the kind was implicitly unbounded). Present with `max_age_days = None` → row existed and was pinned. Present with `max_age_days = Some(n)` → row existed with a finite curve. Required by the §3.1 echo rule so `change_for` reconstructs the byte-exact prior state — without it undo would conflate "no policy" with "policy = forever".

- **`updated_at_ms` echoed inside `prior` snapshot.** Same byte-exact contract used for user.enable's `prior_disabled_at_ms`. The reversible round-trip test (`reversible_round_trip_restores_prior_curve_and_timestamp`) tick-sleeps 5ms between the seed and the operator's mutation so a fresh `NOW()` would diverge — and asserts the restored row carries the original ms timestamp.

- **Idempotency mirror.** `was_unchanged` ↔ existing `was_already_*` pattern. The store's `upsert` contract guarantees `updated_at` is left untouched on a no-op; the verb skips the audit row when `was_unchanged = true` so undo cannot revert an unrelated edit and the redo stack survives under §3.4. Diagnostic: `rubix.audit.policy.unchanged`.

- **Three positive diagnostic codes.** `rubix.audit.policy.set` (finite curve applied), `rubix.audit.policy.pinned` (curve set to NULL = keep forever), `rubix.audit.policy.unchanged` (no-op). Operators reading the audit log can tell at a glance whether retention loosened, tightened, or pinned without inspecting the params.

- **Permissions split.** `audit.policy.read` for `list`, `audit.policy.write` for `set`. Read-only operator roles can inspect the policy without inheriting the destructive write capability.

- **Reversibility self-applies.** The `audit_policy` kind is registered with the dispatcher just like `user`/`team`/`tenant`. So changes to the policy table itself land in `starter_changes`, and operators can `rubix.undo.last` an accidental policy.set. Recursive observation: the `audit_policy` kind itself is not pinned in the seed migration — implicit unbounded. If operators want a long retention curve on policy changes themselves, they pin it via `rubix.audit.policy.set('audit_policy', null)`. Documented in the set descriptor.

- **No Pg-backed store yet.** Registry wires `InMemoryAuditPolicyStore` unconditionally. The Pg impl (mapping to `changelog_kind_policy` directly) is a follow-up slice — same shape as the other in-memory→Pg migrations still pending for `TenantStore`/`UserStore`/`TeamAdminStore`. Listed in "what's next" below.

## Files touched

### New
- `rubix/crates/rubix-spi/src/dto/audit/mod.rs` — goal barrel.
- `rubix/crates/rubix-spi/src/dto/audit/policy_list.rs` — `AuditPolicyListRequest`, `AuditPolicyListResponse`, `AuditPolicyEntry`, descriptor.
- `rubix/crates/rubix-spi/src/dto/audit/policy_set.rs` — `AuditPolicySetRequest`, `AuditPolicySetResponse`, `AuditPolicyPriorSnapshot`, descriptor, `AUDIT_POLICY_KIND` const.
- `rubix/crates/rubix-tools/src/audit/mod.rs` — goal barrel.
- `rubix/crates/rubix-tools/src/audit/store.rs` — `AuditPolicyRow`, `AuditPolicyStore` trait, `InMemoryAuditPolicyStore`, `AuditPolicyReversible` (snapshot shape) + 5 store tests.
- `rubix/crates/rubix-tools/src/audit/policy_list.rs` — `AuditPolicyListTool` (Tool, read-only) + 2 tests.
- `rubix/crates/rubix-tools/src/audit/policy_set.rs` — `AuditPolicySetTool` (Tool + ReversibleTool) + 9 tests.

### Modified
- `rubix/crates/rubix-spi/src/dto/mod.rs` — `pub mod audit;`.
- `rubix/crates/rubix-tools/src/lib.rs` — `pub mod audit;`.
- `rubix/crates/rubix-spi/catalogues/en.json` — 4 new keys (`listed`, `set`, `pinned`, `unchanged`).
- `rubix/crates/rubix-spi/catalogues/es.json` — Spanish equivalents.
- `rubix/crates/rubix-agent/src/registry.rs` — added store, reversible-registry mount, two tool wires (list + reversible set).

## Tests

`cargo test -p rubix-tools --lib audit::` — **16 tests pass**.

Highlights:
- `upsert_with_same_value_is_noop_and_preserves_updated_at` (store) — locks in the idempotency contract that the verb depends on. Includes a 5ms tick-sleep so a non-idempotent impl would visibly diverge.
- `first_set_records_create_draft` (verb) — confirms `Op::Create` shape when the kind was implicitly unbounded.
- `changing_curve_records_update_with_byte_exact_prior` (verb) — locks in the §3.1 echo contract: `prior.max_age_days` and `prior.updated_at_ms` flow through to the `before` snapshot byte-exact.
- `reversible_round_trip_restores_prior_curve_and_timestamp` (verb) — end-to-end: invoke → draft → assemble Change → `AuditPolicyReversible::apply_inverse` → restored row carries the original `updated_at_ms`, not a fresh `NOW()`.
- `reversible_round_trip_undoes_create_by_deleting` (verb) — confirms `Op::Create`'s inverse is `delete`, not `put(null)`.
- `zero_or_negative_max_age_days_is_rejected` — validates the input-domain guard against nonsense retention curves.

## Validation totals

- `cargo build --workspace` — clean (after one `cargo clean` to recover from a full disk; build artefacts were 185 GB-occupying ambient state, not policy-relevant).
- `cargo test -p rubix-tools --lib` — **276 passed** (was 260, +16).
- `cargo test -p rubix-agent --test undo_dispatch_test` — 3/3.
- `cargo test -p rubix-agent --test goal_2_user_admin_test` — 9/9.
- `cargo test -p rubix-agent --test admin_registry_test` — 2/2.
- Clippy clean on audit::* files.

## Admin-session zone

Untouched: `rubix-agent/src/admin/`, `rubix-agent/src/routes/admin/`, `boot/auth.rs`, `crates/starter-changelog-postgres/src/tail_listen.rs`.

## What's next

The in-memory→Pg migration is the largest remaining tranche of work in the rubix admin surface. Recommended slice order (simplest first so each lands with a small blast radius):

1. **`PgAuditPolicyStore` in `rubix-store-postgres`.** Smallest of the four — one table, four trait methods, no membership map, no joins. The `changelog_kind_policy` table already exists; the impl is a thin sqlx layer. Switches the registry to `match pg_pool { Some(pool) => PgAuditPolicyStore, None => InMemoryAuditPolicyStore }` mirroring the dashboard wiring.

2. **`PgRubixTenantStore`.** Mirrors `PgAuditPolicyStore` in shape — single table, no joins. Companion to the existing auth-side `PgTenantStore` (different table — auth-side handles login, rubix-side handles the tool surface). A future slice can debate consolidation.

3. **`PgUserAdminStore`.** Slightly larger — has prefs_json + tenant_id columns, lifecycle includes disabled_at_ms. Still single-table.

4. **`PgTeamAdminStore`.** Largest — membership map needs either a JSON column or a join table. Designed last so the patterns established by 1-3 are settled.

Other candidates (lower priority, can interleave):

- **`rubix.user.delete`.** Hard-delete with refuse-if-referenced (mirror of `tenant.delete`). High risk because user rows are referenced from teams and audit. Needs its own cascade decision and a dedicated session doc.
- **`rubix.audit.policy.clear`.** Delete a policy row entirely (returns the kind to implicit unbounded). Probably not worth the surface — operators can set `max_age_days = Some(VERY_LARGE)` or pin to NULL instead. Punt unless real demand.
- **§3.2 node-level undo** — still gated on Phase B flow_nodes.
- **Warehouse-write Reversibles** — still deferred.

## Cross-references

- `rubix/docs/proposal/audit-log.md` — the audit-log proposal whose mechanism this surface completes.
- `crates/starter-changelog-postgres/migrations/0004_changelog_kind_policy.sql` — the underlying table the policy verbs target.
- [`./2026-05-28-user-enable.md`](./2026-05-28-user-enable.md) — prior slice, established the `prior_*_ms` echo pattern reused here.
- [`./2026-05-28-team-unassign.md`](./2026-05-28-team-unassign.md) — earlier slice that closed the team membership surface.
- [`../../design/undo/README.md`](../../design/undo/README.md) — proposal §3.1 (echo rule) and §3.4 (no-op preserves redo stack).
