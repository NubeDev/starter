# UNDO

> **Current state (2026-05-28):** the production runtime is wired.
> `rubix-agent`'s `main.rs` constructs a `registry::UndoSubstrate`
> from the live PG pool (`PgChangeRecorder` + `PgChangeLog` +
> `PgUndoCursor`), passes it to `build_tool_registry`, which (a)
> wraps every `ReversibleTool` (dashboards, users, teams, flow_ops,
> warehouse — 11 verbs total) in `UndoDispatcher`, (b) appends
> `rubix.undo.last` and `rubix.undo.redo` as callable verbs, and (c)
> applies the `starter_undo_cursors` migration alongside the
> changelog migration so the per-actor redo stack is durable from
> first boot. The REST `/api/v1/tools/*` handler installs the
> caller's `Actor` into a `tokio` task-local so `UndoDispatcher`
> sees the right actor without threading it through every
> `Tool::invoke` signature. The substrate is `Option<…>`: laptops
> without `mcp_url` (no PG) skip the wiring and fall back to
> changelog-only behaviour.


Every reversible write the rubix backend dispatches lands in
`starter_changes`, and any actor can roll back their last group with
`rubix.undo.last`. The wiring has three pieces:

1. **`starter_undo::ReversibleRegistry`** — one
   `starter_spi::changelog::Reversible` impl per resource kind. Built
   once at agent boot and shared as an `Arc` with every dispatcher.
2. **`starter_undo::dispatch::record_if_reversible`** — the helper
   the dispatch wrapper calls after a successful domain mutation.
   Looks up the resource kind in the registry; if found, opens a
   `ChangeRecorder::transaction` and writes one row with the
   `(before, after, op, resource, actor)` the tool supplied.
   Returns the assigned `GroupId`. Unregistered kinds short-circuit
   to `Ok(None)` — read-only verbs and tools that have no Reversible
   counterpart never touch the recorder.
3. **`rubix_tools::undo::dispatch::UndoDispatcher`** — the
   `Tool`-shaped wrapper used at the agent boundary. It calls the
   inner `Tool::invoke`, hands the `(input, output)` pair to the
   tool's `ReversibleTool::change_for` adapter to build a
   `ChangeDraft`, then forwards to `record_if_reversible`. Tools
   that have no Reversible adapter implement `Tool` only and
   bypass the wrapper.

The verb that closes the loop is **`rubix.undo.last`**
(`rubix_tools::undo::last::UndoLastTool`). It pulls the calling
`Actor` from an `ActorSource` (the agent loop's request context) and
calls `starter_undo::undo_last(service, actor, scope)`, which today
delegates to `UndoService::undo` and walks the actor's most recent
group. The `scope` parameter is reserved for a per-resource filter
the goal-2/3/4 work introduces; the verb already accepts it so the
client contract does not change when the filter activates.

## Adding a new reversible resource

1. Implement `starter_spi::changelog::Reversible` for the resource
   and register the impl with `ReversibleRegistry::insert` at boot.
2. Implement `ReversibleTool::change_for` on the tool that mutates
   it; return `Some(ChangeDraft)` describing the before/after
   snapshot pair.
3. Wrap the tool with `UndoDispatcher::new(inner, registry,
   recorder, actor)` in the agent's tool registry.

Nothing else changes — the dispatcher, helper, and `rubix.undo.last`
verb are kind-agnostic.

## Snapshot vs patch policy

See the rustdoc on `starter_spi::changelog::Reversible` for the full
decision matrix; the short version: snapshot is the default, patch is
the escape hatch when the row is large or the delta is small. Each
in-tree impl tags its choice (see the policy section of its module
doc).

## Dashboard metadata fold (proposal §3.1 decision)

`rubix.dashboard.update` and `rubix.dashboard.patch` write the page
body **and** carry title/tags. There is no separate
`rubix.dashboard.definition` verb today. The Reversible impl folds
metadata into the page snapshot: `DashboardSnapshot` carries
`title`, `tags`, and `body_json` together, so undo of a rename
restores all three atomically. The chokepoint
`DashboardStore::insert_revision_with_prior` returns the superseded
row's metadata in `prior_title` / `prior_tags`, which `change_for`
threads into the `before` snapshot.

This is a pragmatic fold rather than the architecturally cleaner
"separate `rubix.dashboard.definition` kind" the proposal mentions
as an alternative. Revisit when either:

1. A metadata-only verb appears (today only `update` can touch
   title/tags, and it always writes the body too), or
2. A "rename history" view wants to query metadata edits in
   isolation from body edits.

Until then, the folded snapshot is correct and round-trips byte-exact.

## Outstanding gaps

- **Warehouse write verbs have no Reversible impls.** The four
  TimescaleDB-backed writes — `rubix.warehouse.rule.write`,
  `rubix.warehouse.mart.create`, `rubix.warehouse.mart.drop`,
  `rubix.warehouse.retention.set` — return `prior_ddl` /
  `prior_days` in their response payloads (so callers can snapshot
  externally) but no `impl Reversible` exists for them and the
  values are not persisted to `undo_snapshots`. Snapshot shapes are
  documented in
  [`../warehouse-rules/README.md`](../warehouse-rules/README.md);
  the impls themselves are deferred until an operator demand
  surfaces — until then, warehouse writes are advertised as
  one-way in the verb descriptors.
- **User role / prefs Reversible.** Proposal §3.3 explicitly
  defers this until a separate audit-log proposal exists. Don't
  extend undo retention to substitute for audit. The audit-log
  proposal now exists at
  [`../../proposal/audit-log.md`](../../proposal/audit-log.md) —
  it landed the mechanism (steps 1–4 of the proposal's "Concrete
  next steps") so §3.3 itself is now an isolated change:
  - `changelog_kind_policy` table on `starter_changes` (provisioned
    by `crates/starter-changelog-postgres/migrations/0004_…sql`).
  - `starter_changelog_postgres::apply_policy` sweep helper.
  - `rubix-agent::boot::changelog_sweep` (mirror of `undo_sweep`),
    spawned from `main.rs`.
  - Rubix-side seed
    (`rubix/crates/rubix-store-postgres/migrations/changelog_policy/0001_…sql`)
    pins `user` and `team` to `max_age_days = NULL` (keep forever).
    The audit floor is recorded in SQL, not tribal memory.
  §3.3 is **closed** (2026-05-28). All three verbs landed:
  - `rubix.user.role.set` — see
    [`../../sessions/undo/2026-05-28-user-role-set.md`](../../sessions/undo/2026-05-28-user-role-set.md).
  - `rubix.user.prefs.set` — see
    [`../../sessions/undo/2026-05-28-user-prefs-set.md`](../../sessions/undo/2026-05-28-user-prefs-set.md).
    Also fixed the §3.1-bug-class snapshot bug that was silently
    clearing identity-bearing fields on undo.
  - `rubix.user.tenant.assign` — see
    [`../../sessions/undo/2026-05-28-user-tenant-assign.md`](../../sessions/undo/2026-05-28-user-tenant-assign.md).
    Model change: `UserRow` gained `tenant_id: Option<String>`;
    the verb validates the tenant resolves in `TenantStore` before
    writing (no silent FK violation). Cascade-on-tenant-delete is
    out of scope today — there is no tenant-delete verb — and the
    decision is recorded in the verb's DTO doc so it gets debated
    rather than implicitly made.

  ### User account-state symmetry (`user.enable`)

  Shipped 2026-05-28 closing the cross-actor re-enable hole.
  `rubix.undo.last` is per-actor — admin A could undo their own
  `disable`, but admin B could not re-enable a user A had
  disabled. `rubix.user.enable` is the canonical re-enable
  surface (same `users.write` permission, idempotent mirror of
  `disable`, byte-exact restoration of the prior
  `disabled_at_ms` timestamp on the `before` snapshot). See
  [`../../sessions/undo/2026-05-28-user-enable.md`](../../sessions/undo/2026-05-28-user-enable.md).

  ### Audit policy operator surface (`audit.policy.list` + `audit.policy.set`)

  Shipped 2026-05-28 closing the "policy is configurable only
  via SQL seed" gap. `changelog_kind_policy` has had operator
  significance since the audit-log proposal landed but no live
  surface; the two new verbs let operators inspect and mutate
  per-kind retention with full reversibility. New
  `AuditPolicyReversible` for the `audit_policy` kind (snapshot
  shape, full-row payload incl. `updated_at_ms` for byte-exact
  restoration). Three positive diagnostic codes
  (`set` / `pinned` / `unchanged`) so an operator reading the
  audit log can tell whether retention loosened, tightened, or
  pinned without inspecting params. Policy changes are
  themselves audited under the `audit_policy` kind (recursive
  observation — operators can pin `audit_policy` retention via
  the same verb).
  See [`../../sessions/undo/2026-05-28-audit-policy.md`](../../sessions/undo/2026-05-28-audit-policy.md).

  ### Tenant lifecycle (`tenant.create` / `tenant.update` / `tenant.delete`)

  Landed 2026-05-28 alongside the §3.3 close. Adds a new
  `TenantReversible` (snapshot shape, full-row payload) for the
  `tenant` kind and pins the kind to the audit floor
  (`changelog_policy/0002_tenant_audit_floor.sql`). All three
  verbs route through the single `TenantReversible`; the
  `apply_inverse` matrix covers `Op::Create` / `Op::Update` /
  `Op::Delete` symmetrically. See:

  - [`../../sessions/undo/2026-05-28-tenant-lifecycle.md`](../../sessions/undo/2026-05-28-tenant-lifecycle.md)
    — cascade-on-delete decision (refuse-if-users-assigned)
    and the alternatives considered.
  - [`../../sessions/undo/2026-05-28-tenant-update.md`](../../sessions/undo/2026-05-28-tenant-update.md)
    — rename + relocale design, immutable-id rationale,
    self-rename regression guard.

  Notably:

  - `tenant.create` is **not** silent-idempotent — duplicate id
    or name returns `Error::Conflict`. Tenants are identity
    boundaries; silent idempotency would let two operators think
    they each "own" a tenant they share.
  - `tenant.update` mutates `name` and/or `locale` only — id is
    immutable. Per-field optionality; "update with no fields"
    is rejected as `Error::Invalid` rather than collapsed to
    unchanged. Rename uniqueness is enforced at the verb level
    (uniqueness-excluding-self) so the store trait stays at
    two write methods.
  - `tenant.delete` refuses with the structured
    `rubix.tenant.has_users` diagnostic when any `UserRow`
    carries `tenant_id == target`. Operator unassigns first.
  - Undo of `delete` restores the tenant row but does NOT
    re-attach previously assigned users — those assignments live
    in their own undo chain (`rubix.user.tenant.assign`) and
    were unwound separately before the delete.

  ### Team lifecycle (`team.create` / `team.update` / `team.delete` / `team.assign` / `team.unassign`)

  Team CRUD landed 2026-05-28 in three slices: morning
  (`team.create` + `team.assign`), midday (`team.update` +
  `team.delete`), and the unassign closeout below. All five
  verbs route through the same `TeamReversible` (patch-shape
  payload — `assign`/`unassign` mutate only `members`, `update`
  mutates only `name`/`description`, so patches address
  disjoint fields and undo can't clobber concurrent edits).
  `delete` uses snapshot shape (`Op::Delete` carries the full
  row including members), matching
  `TeamReversible::apply_inverse`'s existing `parse_row` arm.

  See:
  - [`../../sessions/undo/2026-05-28-team-crud-closeout.md`](../../sessions/undo/2026-05-28-team-crud-closeout.md)
    — cascade-on-delete decision (allow-with-disclosure, contrasted with tenant's refuse-if-users-assigned), trait-surface changes (`TeamAdminStore::list` added, `delete` made strict).
  - [`../../sessions/undo/2026-05-28-team-unassign.md`](../../sessions/undo/2026-05-28-team-unassign.md)
    — closes the cascade-on-delete footgun by giving operators a drain-before-delete path. Locks in the patch-shape contract via the `undo_preserves_concurrent_rename` regression test.

  - `team.update` mutates `name` and/or `description` only — id
    is immutable. Patch carries only the flipped fields, so
    membership stays untouched on undo. Rename uniqueness is
    enforced verb-side (uniqueness-excluding-self).
  - `team.delete` cascades through the membership map; the
    operator sees the cascaded member count in the diagnostic
    and undo restores members byte-exact.
  - `team.unassign` is idempotent on missing members
    (`already_not_member = true`, no draft) but hard-errors on
    missing teams. Reversible round-trip preserves the original
    `assigned_at_ms` timestamp.

## Tests

- **`starter_undo::dispatch::tests`** — unit-level round-trip
  through a fake `Reversible` and an in-memory recorder.
- **`rubix_agent` integration test `undo_dispatch_test.rs`** —
  three tests: (1) registers a fake tool + Reversible, dispatches
  through the live `SqliteChangeRecorder`, asserts the recorded row
  drives the inverse path; (2) pins the proposal §3.4
  redo-clear-on-mutation invariant; (3) guards that unregistered
  kinds don't touch the cursor.
- **`starter_undo::tests::cursor_postgres`** — five
  docker-backed tests covering the `PgUndoCursor` epoch CAS:
  round-trip, isolation, persistence, concurrent CAS, and
  agent-run-id keying. Run with
  `cargo test -p starter-undo --features postgres --test cursor_postgres -- --ignored`.
- **`rubix_agent::tests::undo_redo_e2e_test`** — single
  docker-backed end-to-end pinning the create → update → undo →
  redo → clear-on-mutation → cross-process-replay sequence through
  `UndoDispatcher::with_cursor` + `PgUndoCursor` + the
  `rubix.undo.{last,redo}` verbs. Run with
  `cargo test -p rubix-agent --test undo_redo_e2e_test -- --ignored`.

Both `--ignored` suites are wired into the CI `undo-postgres` job
([`.github/workflows/ci.yml`](../../../../.github/workflows/ci.yml))
so a regression in the cursor or dispatch wiring fails the PR check
instead of waiting on a developer remembering `-- --ignored` locally.

