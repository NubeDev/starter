# Proposal: Append-only audit log for security-relevant writes

Status: Draft (v1, revised 2026-05-28 after re-reading the storage layout)
Owner: ap@nube-io.com
Date: 2026-05-28

## Summary

The undo/redo proposal
([`flow-storage-and-undo-redo.md`](flow-storage-and-undo-redo.md) §3.3)
defers the `UserReversible` extension for role / prefs / tenant-assignment
until "a separate audit-log proposal exists." This is that proposal.

**Recommendation: do not build a new audit system. The audit signal
already exists and survives undo retention.** Walking the actual
storage layout:

| Table | Owner | Sweeps in rubix? | Read API |
|---|---|---|---|
| [`starter_changes`](../../../crates/starter-changelog-postgres/migrations/0001_init.sql) | `starter-changelog-postgres` | **No** | `GET /v1/audit` via [`starter-audit`](../../../crates/starter-audit/src/routes.rs) |
| [`undo_snapshots`](../../crates/rubix-store-postgres/migrations/undo/0001_undo_snapshots.sql) | `rubix-store-postgres` | **Yes** ([`undo_sweep.rs`](../../crates/rubix-agent/src/boot/undo_sweep.rs)) | `rubix.undo.last` / `rubix.undo.redo` |

Every reversible mutation lands a row in **both**: the rubix tool's
`Reversible::change_for` produces a snapshot row in `undo_snapshots`
(for the inverse op), and `UndoDispatcher` calls
`record_if_reversible` which writes a `Change` row to `starter_changes`
(for the audit trail). The two tables answer different questions and
have different lifetimes:

- `undo_snapshots` is the **operational** record — "give me the prior
  body so I can replay it." Bounded retention is correct here; a
  6-month-old undo is useless and the table would grow without bound.
- `starter_changes` is the **historical** record — "who changed what
  when." rubix runs no sweep against it. `starter-changelog-postgres`
  ships a `Prune` trait
  ([`prune.rs`](../../../crates/starter-changelog-postgres/src/prune.rs)),
  but it's caller-driven; no rubix boot path invokes it. Effectively
  unbounded today.

So the §3.3 concern — "extending undo retention to substitute for
audit is wrong-shaped" — is already structurally avoided. Pruning
`undo_snapshots` only loses the *undo capability*, not the audit row.
The `Change` in `starter_changes` persists; `starter-audit` returns
it via `GET /v1/audit?actor_kind=user&resource_kind=user&…`.

The actual gaps for §3.3 are smaller and more focused than the parent
proposal implied:

1. **`Change` does not yet carry role / prefs / tenant_id deltas.**
   `UserReversible::change_for` produces snapshots for create and
   disable today; role and prefs writes don't flow through it because
   no rubix tool exposes them. Until those tools land, the audit row
   for "alice demoted bob" doesn't get written — *the table is fine,
   the producer is missing*.
2. **No explicit operator policy for `starter_changes` retention.**
   The table is unbounded by default; an operator who decides to
   trim it for storage reasons today has no per-kind dial — they
   must use the raw `Prune` trait against the whole table. For
   security-relevant kinds we want the policy to be *opt-in
   permanent retention* enforced in code, not "operator
   remembers not to prune `user` rows."
3. **No frontend surface.** `GET /v1/audit` works; nothing renders
   it. Out of scope for this proposal — UI work follows the §3.3
   verb extension.

## Non-goals

- **Cryptographic signing / Merkle log.** Out of scope until a
  compliance regime asks for it (SOC2 Type II, ISO 27001 chain of
  custody). Trusted-DBA model is sufficient for v1: the PG row is the
  authoritative record, write access is gated by deploy infra.
- **External SIEM forwarding.** A tail consumer can subscribe to
  `starter-changelog`'s LISTEN/NOTIFY channel today
  ([`starter-changelog-postgres::tail_listen`](../../../crates/starter-changelog-postgres/src/tail_listen.rs)).
  Forwarding to a vendor (Datadog, Splunk) is wiring, not a proposal.
- **Replacing `starter-authz`'s decision audit.** That covers a
  different signal: "was this request allowed?" rather than "what data
  changed?" Both stay; they answer adjacent questions.
- **Per-field redaction.** v1 stores the full `before` / `after`
  snapshot as today. A subsequent proposal can introduce kind-level
  redaction rules (e.g., drop `password_hash` from `before` even
  though the column exists). Not required to land §3.3.
- **A new audit table.** Forking the changelog would double the write
  cost, create a sync problem, and force `starter-audit` to UNION two
  sources. The changelog row IS the audit row.

## Existing landscape (what already works)

### Write path: `starter-changelog`

`starter_undo::dispatch::record_if_reversible` opens a
`ChangeRecorder::transaction` and writes one row with `actor`,
`resource_kind`, `op`, `before`, `after`, `group_id`. The PG impl
NOTIFY-emits on each insert so tail consumers see new rows in
milliseconds. Wired through `UndoDispatcher::invoke_with_group` for
every reversible verb in rubix-agent's registry.

### Read path: `starter-audit`

`AuditService::list`
([`crates/starter-audit/src/lib.rs`](../../../crates/starter-audit/src/lib.rs))
returns a paged projection of `starter_changes` filtered to
`actor_kind = 'user'`, after passing every row through
`ChangelogVisibilityRegistry` so a confused-deputy read can't leak
another tenant's rows. The router lives at
[`crates/starter-audit/src/routes.rs`](../../../crates/starter-audit/src/routes.rs):
`GET /v1/audit?…ChangeFilter`. Frontend can already paginate
"who-changed-what-when" today; visibility gate is unchanged.

### Decision audit: `starter-authz`

Separate concern: `DecisionSink`
([`crates/starter-authz/src/audit/db.rs`](../../../crates/starter-authz/src/audit/db.rs))
records every allow/deny verdict from the policy evaluator. Sampled
(1-in-100 allows, 100% denies), bounded queue, retention-pruned.
This is the answer to "did AuthZ let this happen?" — not "did the
data change?"

## Proposed change

### Mechanism: explicit per-kind retention policy for `starter_changes`

Today there is no per-kind policy table for the changelog. Add a
small one, mirroring `undo_kind_policy` but living next to the
changelog because that's the table it governs:

```sql
CREATE TABLE changelog_kind_policy (
    resource_kind   TEXT PRIMARY KEY,
    max_age_days    INT,            -- NULL = unbounded retention
    -- Room to grow: per-tenant overrides, redaction rules, export hooks.
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

Semantics:

- A kind with no row in `changelog_kind_policy` → unbounded retention
  (today's behaviour, no surprise).
- A row with `max_age_days = NULL` → explicit "keep forever" — used
  for the security-relevant kinds (`user`, `team`) and for any kind
  an operator wants on the audit floor.
- A row with `max_age_days = N` → an operator opted into bounded
  retention for that kind (e.g., a chatty `flow_def` history that
  doesn't need 7 years of edits).

Add a sweep at the rubix-agent boot path that mirrors
`undo_sweep.rs` but targets `starter_changes` and only deletes when
the policy row says `max_age_days IS NOT NULL`. Defaults preserve
today's behaviour: nothing is deleted unless an operator opts in.

Seed the security-relevant kinds with explicit unbounded rows so
the *intent* is recorded in SQL, not just absence:

```sql
INSERT INTO changelog_kind_policy (resource_kind, max_age_days) VALUES
    ('user',   NULL),   -- explicit: keep forever (audit floor)
    ('team',   NULL);   -- explicit: keep forever (audit floor)
-- 'flow_def', 'dashboard_page', etc. left unspecified — implicit
-- unbounded today; operator can add a row to opt into pruning.
```

The seed isn't enforcing anything new — it's documenting the
existing unbounded retention as a deliberate choice for these
kinds, so a future operator who adds a sweep can't accidentally
prune them without first updating policy.

### What does NOT need to change

- `starter_changes` schema — unchanged.
- `undo_snapshots` schema — unchanged.
- `undo_kind_policy` — unchanged; continues to govern undo lifetime.
- `record_if_reversible` — unchanged.
- `starter-audit` read path — unchanged; readers see whatever
  survived in `starter_changes`.
- `PgUndoCursor` — unchanged; redo stack still expires on the undo
  curve.

### Why not extend `undo_kind_policy` instead

I considered adding `audit_only_max_age_days` to `undo_kind_policy`
in a draft of this proposal. Rejected because:

- The two tables live in different crates (`rubix-store-postgres`
  vs `starter-changelog-postgres`). Coupling them makes
  `starter-changelog` depend on rubix policy semantics — wrong
  direction, since `starter-audit` is workspace-generic.
- The policies answer different questions: undo retention is a UX
  knob (how far back can a user reverse?), changelog retention is
  a compliance knob (how far back can audit see?). Co-locating them
  invites confusion of the two timescales.

## What this unblocks for proposal §3.3

With the `changelog_kind_policy` table in place and `user` / `team`
pinned to unbounded, the §3.3 work is a straightforward
`UserReversible` + verb extension:

1. Add `role`, `prefs_json`, `tenant_id` to `UserRow` (or split
   into `UserRoleReversible` / `UserPrefsReversible` if the
   user-admin verbs grow into distinct tools — TBD when the next
   user-side verb lands).
2. Populate `before` / `after` in the `change_for` adapter so the
   `Change` row carries the role/prefs delta.
3. Add the verbs (`rubix.user.role.set`, `rubix.user.prefs.set`,
   `rubix.user.tenant.assign`) and wire through `UndoDispatcher`
   like every other reversible verb.

No new audit retention dance per kind, no parallel persistence
path, no "should the undo system also be the audit log" debate.
The changelog *is* the audit log; per-kind policy decides how long
each slice persists, and the security-relevant kinds are pinned
to unbounded.

## Concrete next steps

1. Migration in `starter-changelog-postgres` (new file under
   `crates/starter-changelog-postgres/migrations/`) adding
   `changelog_kind_policy`. Defaults preserve today's unbounded
   behaviour for every kind.
2. New module `starter-changelog::policy` exposing
   `apply_policy(pool) -> PruneReport` that deletes from
   `starter_changes` only for kinds whose policy row has
   `max_age_days IS NOT NULL`. Sqlite twin lands when needed.
3. New rubix-agent boot module `boot/changelog_sweep.rs` mirroring
   `undo_sweep.rs` (boot tick + 24h ticker) calling the new helper.
   `Option<JoinHandle<()>>` return so the laptop path skips
   cleanly when no PG pool exists.
4. Seed migration in `rubix-store-postgres` (next free number under
   `migrations/undo/`, or a new `migrations/changelog/` dir if we
   want the seed separate from the table) inserting the
   unbounded-`user`/`team` rows.
5. Ship §3.3 per the parent undo proposal: extend `UserReversible`
   for role / prefs / tenant-assignment and add the three verbs.
6. Update [`rubix/docs/design/undo/README.md`](../design/undo/README.md)
   so the "undo is not audit" rule has a corresponding "and the
   audit floor lives in `changelog_kind_policy`" pointer.

## Open questions

- **Per-tenant retention overrides.** Today's sketch is global. A
  tenant on a stricter compliance contract may need a per-tenant
  override. Defer until the first tenant asks; the shape is "add
  `(tenant_id, resource_kind)` composite PK with a NULL row meaning
  'all tenants'."
- **Audit-row redaction for GDPR right-to-erasure.** A user
  exercising erasure has the right to have their data scrubbed —
  including from the audit log, modulo legitimate-interest
  retention. v1 punts: the audit row survives intact. A follow-up
  proposal can introduce a `redact_pii(change_row)` hook that runs
  on erasure-request processing.
- **Surface in the UI.** `GET /v1/audit` is consumer-ready but
  unrendered. Out of scope; UI work follows the §3.3 extension when
  the role / prefs writes start flowing through.
- **Should `flow_def` be opted into bounded retention?** Authoring
  churn produces a lot of revisions. Discuss with the flow team
  before defaulting either way. Easiest path: ship the table empty
  for `flow_def` (implicit unbounded) and revisit if the row count
  bites.
