# user-admin

Present-tense design note for the rubix user-admin goal. Covers the
four write verbs that landed in phase B.1 — `rubix.user.create`,
`rubix.user.disable`, `rubix.team.create`, and `rubix.team.assign` —
plus the snapshot shape every verb's `Reversible` impl reads and
writes.

## Verb surface

| Verb               | Op       | MessageKey(s)                                              | Reversible? |
| ------------------ | -------- | ---------------------------------------------------------- | ----------- |
| `rubix.user.create`  | Create | `rubix.user.created`                                       | yes         |
| `rubix.user.disable` | Update | `rubix.user.disabled`, `rubix.user.already_disabled`       | yes (first-disable only — the idempotent re-call records no Change) |
| `rubix.team.create`  | Create | `rubix.team.created`                                       | yes         |
| `rubix.team.assign`  | Update | `rubix.team.assigned`                                      | yes (first-assign only — the idempotent re-call records no Change) |

Each verb implements `starter_spi::tool::Tool` for invocation and
`rubix_tools::undo::dispatch::ReversibleTool` for the `change_for`
adapter that emits an optional `starter_undo::ChangeDraft` from the
`(input, output)` pair on the success path. The dispatcher
(`UndoDispatcher`) forwards the draft to
`starter_undo::record_if_reversible`, which records the change
through the workspace `ChangeRecorder` so a later `rubix.undo.last`
walks it back via the per-kind `Reversible` impl.

## Backing store

Both goals talk to a small per-kind trait —
`rubix_tools::user::store::UserAdminStore` and
`rubix_tools::team::store::TeamAdminStore`. The trait is a thin
seam so the production binary can swap a PG-backed impl in without
touching the verb files. Today the only impl is the in-memory
`InMemoryUserStore` / `InMemoryTeamStore` used by the unit tests
and the agent-loop integration tests; the PG impl wiring lands in
a follow-up stage that consumes `starter-auth-users` plus the new
`teams` migration.

## Snapshot shape

`starter_spi::changelog::Change` carries an `Op` plus optional
`before` / `after` JSON payloads. Each rubix resource kind owns the
JSON layout for those payloads; the `Reversible` impl interprets
them.

### `kind = "user"`

`Op::Create`:

- `before`: `null`
- `after`: full `UserRow` JSON (`user_id`, `email`, `role`,
  `disabled_at_ms`).
- Inverse: `store.delete(user_id)`.

`Op::Update` (disable):

- `before`: full `UserRow` JSON with `disabled_at_ms = null`.
- `after`: full `UserRow` JSON with `disabled_at_ms = Some(epoch_ms)`.
- Inverse: `store.put(before)` — restores the prior row verbatim.
- The verb echoes `role` on the response so the snapshot can
  reconstruct the full row without a follow-up read.

### `kind = "team"`

`Op::Create`:

- `before`: `null`
- `after`: full `TeamRow` JSON (`team_id`, `name`, `description`,
  `members: BTreeMap<user_id, assigned_at_ms>`).
- Inverse: `store.delete(team_id)`.

`Op::Update` (assign): **sparse patch**, not a full row.

- `before` / `after`: `TeamPatch { members?: BTreeMap, name?: String,
  description?: Option<String> }`. Only the fields the verb actually
  touched are populated — assign sets `members` and leaves the rest
  `None`.
- Inverse: read the current `TeamRow`, overlay the `before` patch,
  `store.put` the merged row. The merge skips fields the verb did
  not touch so concurrent edits to unrelated fields are preserved.

The sparse-patch shape is what lets the goal stay reversible
without forcing the verb response DTOs to carry an entire prior
row. The agent loop's `change_for` adapter stashes the patch
payloads on the response JSON under two reserved keys —
`_prior_members` and `_new_members` — that the typed REST DTO
ignores; the keys are an implementation detail of the dispatch
pipeline, not a public surface.

## Localisation

Five MessageKeys land alongside the verb files:

- `rubix.user.created` — `{email}`, `{role}`, `{at}`
- `rubix.user.disabled` — `{email}`, `{at}`
- `rubix.user.already_disabled` — `{email}`, `{at}`
- `rubix.team.created` — `{name}`, `{at}`
- `rubix.team.assigned` — `{team}`, `{user}`, `{at}`

Entries land in both `rubix-spi/catalogues/en.json` and
`rubix-spi/catalogues/es.json` in the same commit that fills the
verbs (workspace rule R5).

## Idempotence and undo

`user.disable` and `team.assign` are idempotent — re-calling them
against an already-disabled user or an already-assigned membership
returns the same outcome code (with a `was_already_disabled` /
`already_member` flag set on the typed response) and produces
**no** `ChangeDraft`. The dispatcher's `change_for` returns `None`
on the no-op path, so undo can never silently unwind a state the
caller did not actually flip.

`user.create` and `team.create` always produce a draft on success;
their inverse is the corresponding delete.
