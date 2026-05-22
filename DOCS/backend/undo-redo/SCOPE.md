# Undo / Redo / Audit / Agent-log / Duplicate / Paste — Scope

> Status: proposed. No code yet. This document defines the boundary,
> the seam, and the non-goals before any crate is added.

## One-line summary

Five product features — **user audit log, AI-agent log, undo/redo history,
duplicate, copy/paste** — collapse onto **one primitive**: a typed,
append-only **change envelope** the consumer emits whenever a domain
mutation happens. Starter owns the envelope, the storage, and the
dispatch; the consumer owns their schema and implements one trait per
resource kind to make their data participate.

## Why this belongs in `starter`

Every consumer project re-rolls the same plumbing badly:

- Hand-rolled `audit_log` tables that grow a column per feature.
- Agent runs that mutate data with no replay or attribution.
- Undo stacks built in the UI that desync from the server.
- "Duplicate" buttons that hand-copy fields and forget the new ones.
- Copy/paste across sessions that can't survive a reload.

Each is small. The cost is consistency. Pulled into a starter crate set,
the consumer wires one trait per resource and gets all five features at
once — without forking starter when they add a sixth resource.

## Hard rules (inherits from repo `SCOPE.md`)

- **R2** — the envelope and traits live in `starter-spi`. Zero deps.
- **R4** — one starter-owned table per backend, namespaced migration.
  No `Store` trait. The payload column is `JSON`/`jsonb`; starter does
  not know the consumer's schema.
- **R3** — REST/gRPC/MCP/CLI handlers are thin: extract → call domain
  inside `recorder.transaction(|tx| ...)` → return. Undo/redo/paste
  endpoints dispatch through a registry; transport code knows nothing
  about resource kinds.
- **R5** — every crate listed below is opt-in. A consumer wanting only
  audit pulls `starter-changelog` + one backend + `starter-audit` and
  pays nothing for undo or clipboard.
- **R1** — one crate per concern. No `starter-history` god-crate.

## The seam (in `starter-spi`)

Types and traits below live behind a `changelog` feature on `starter-spi`.
They reuse existing spi types — `ResourceRef` from `starter_spi::authz`,
`Principal::subject` for the user actor id, and `starter_spi::Error`
(with its existing `Conflict { message }` variant for stale undo). No
new error enum, no new resource-ref shape.

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use starter_spi::authz::ResourceRef;
use starter_spi::Result;

/// ULID newtypes. Owned by this module — not generic `Id<T>`, because
/// the table is starter-owned and the id space is shared across kinds.
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ChangeId(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct GroupId(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)] pub struct TraceId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Actor {
    /// Reuses `Principal::subject`. No parallel `principal_id` field.
    User   { subject: String },
    Agent  { run_id: String, model: String },
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Create,
    Update,
    Delete,
    /// `String`, not `&'static str` — values are read back from `jsonb`.
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub id: ChangeId,                       // ULID, monotonic
    pub at: DateTime<Utc>,
    pub actor: Actor,
    pub resource: ResourceRef,              // reused from `starter_spi::authz`
    /// Optimistic-concurrency token taken at read time. `apply_inverse`
    /// uses it as the `WHERE` predicate and returns `Error::Conflict`
    /// if the row has moved on. `None` for resources without
    /// versioning (caller accepts last-write-wins).
    pub resource_version: Option<u64>,
    pub op: Op,
    pub before: Option<serde_json::Value>,  // for undo
    pub after:  Option<serde_json::Value>,  // for redo / duplicate / paste
    /// Optional RFC 6902 patch document, as raw JSON. Kept as
    /// `serde_json::Value` so `starter-spi` does not depend on a
    /// patch-library crate (the backend picks one).
    pub patch:  Option<serde_json::Value>,
    /// Every change belongs to a group. Single-row mutations get a
    /// fresh `GroupId`; multi-row transactions share one. Undo
    /// operates on whole groups, so this is never `Option`.
    pub group_id: GroupId,
    pub correlation: Option<TraceId>,
}

/// The ONLY way to record changes. There is no top-level `record()`;
/// all writes go through a transaction so `group_id` is assigned
/// once and correct grouping is the easy default.
#[async_trait]
pub trait ChangeRecorder: Send + Sync {
    async fn transaction<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: for<'tx> FnOnce(&'tx dyn ChangeTx) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send;
}

#[async_trait]
pub trait ChangeTx: Send + Sync {
    fn group_id(&self) -> &GroupId;
    async fn record(&self, ch: Change) -> Result<ChangeId>;
}

/// Implemented by the consumer, once per resource kind. The ONLY
/// extension point.
///
/// Errors: return `starter_spi::Error::NotFound` if the target row is
/// gone, and `Error::Conflict { message }` if `resource_version`
/// doesn't match the current row (the message SHOULD include the
/// observed version so the UI can render a meaningful refusal).
#[async_trait]
pub trait Reversible: Send + Sync {
    fn kind(&self) -> &'static str;

    /// Undo. Implementations MUST honor `ch.resource_version` when
    /// the resource supports versioning.
    async fn apply_inverse(&self, ch: &Change) -> Result<()>;

    /// Redo / paste.
    async fn apply_forward(&self, ch: &Change) -> Result<()>;

    /// Duplicate / paste-as-new. Returns `Vec<ResourceRef>` because a
    /// composite resource (dashboard + widgets, doc + sections) maps
    /// to N new rows. The implementation is responsible for running
    /// its own DB transaction and emitting one `record()` per new
    /// row inside `ChangeRecorder::transaction` so they all share one
    /// `group_id` and undo collapses them into a single step.
    async fn clone_with(
        &self,
        tx: &dyn ChangeTx,
        src: &ResourceRef,
        overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>>;
}
```

Registration: `ReversibleRegistry::insert(impl Reversible)` builder on
the server, consumed by the undo / paste handlers via a typed
`ResourceKind -> &dyn Reversible` lookup. Transports stay thin (R3) —
they only know `(kind, id)`, never `match` on kinds.

That is the entire surface. New resource kind = one `Reversible` impl,
registered at server build time. No fork of any starter crate.

## Crates

Each does one job (R1) and is independently optional (R5).

| Crate | Job | Depends on |
|---|---|---|
| `starter-changelog` | `ChangeRecorder` / `ChangeTx` traits, `Change` type, `ChangeLog` query API (filter by actor / resource / time / `group_id`), `prune` CLI, default `ChangelogVisibility` registry. No SQL. | `starter-spi` |
| `starter-changelog-sqlite` | One table `starter_changes`, namespaced migration, `ChangeRecorder` impl. | `starter-changelog`, `starter-store-sqlite` |
| `starter-changelog-postgres` | Same shape; `jsonb` payload, optional `LISTEN/NOTIFY` tail. | `starter-changelog`, `starter-store-postgres` |
| `starter-audit` | Read-only projection over `ChangeLog` filtered to `Actor::User`. REST + CLI. Enforces read ACLs via the consumer's `ChangelogVisibility` impl (see Security). | `starter-changelog`, `starter-server` |
| `starter-agent-log` | Read-only projection filtered to `Actor::Agent`. The join key is the opaque `Actor::Agent::run_id` string; consumers that have `starter-ai` wire the join themselves. No starter-ai dep. | `starter-changelog` |
| `starter-undo` | Per-actor undo/redo cursor over `starter_changes` grouped by `group_id`. Dispatches through the `Reversible` registry. | `starter-changelog` |
| `starter-clipboard` | Copy = persist `after` of a `ResourceRef` into `starter_clipboard` (HMAC-signed with a key fetched via `starter_spi::SecretStore` under `starter.clipboard.hmac`, TTL'd, scoped to principal). Paste = `Reversible::clone_with`. Duplicate = copy+paste in one call. Owns its own backend tables and migrations. Depends only on the spi trait — the consumer wires the concrete secrets backend. | `starter-changelog` |

No crate above knows what a "note" or a "dashboard" is. The consumer's
domain code is where kinds are defined and `Reversible` is implemented.

## Storage shape (sketch, same logical schema both engines)

Two separate migrations owned by two separate crates — the changelog
backends own `starter_changes`; `starter-clipboard-{sqlite,postgres}`
owns `starter_clipboard`. They are listed together only because a
consumer pulling both will run both migrations on the same DB.

### Owned by `starter-changelog-{sqlite,postgres}`

```
starter_changes
  id               TEXT/UUID PRIMARY KEY        -- ULID
  at               TIMESTAMPTZ NOT NULL
  actor_kind       TEXT NOT NULL                -- 'user' | 'agent' | 'system'
  actor_id         TEXT                         -- principal subject or agent run_id
  actor_meta       JSON/JSONB                   -- e.g. { model: "claude-..." }
  actor_model      TEXT GENERATED               -- (PG) extracted from actor_meta->>'model'
                                                -- (SQLite) plain column, written by recorder
  resource_kind    TEXT NOT NULL
  resource_id      TEXT NOT NULL
  resource_version BIGINT                       -- optimistic-concurrency token
  op               TEXT NOT NULL
  before           JSON/JSONB
  after            JSON/JSONB
  patch            JSON/JSONB
  group_id         TEXT NOT NULL                -- assigned by ChangeRecorder::transaction
  correlation      TEXT
  INDEX (resource_kind, resource_id, at DESC)
  INDEX (actor_kind, actor_id, at DESC)
  INDEX (actor_kind, actor_model, at DESC)     -- agent-log filters by model
  INDEX (group_id)
```

Note: no `causation` FK and no self-reference. `group_id` is generated
by `ChangeRecorder::transaction` *before* the first `record()` inside
the closure, so every row in the group — including the first — carries
the same value. Undo loads a group by `WHERE group_id = ?` and applies
inverses in reverse `at` order.

### Owned by `starter-clipboard-{sqlite,postgres}`

```
starter_clipboard
  id            TEXT PRIMARY KEY
  principal_id  TEXT NOT NULL
  resource_kind TEXT NOT NULL
  payload       JSON/JSONB NOT NULL          -- the `after` snapshot
  signature     BLOB/BYTEA NOT NULL          -- HMAC over (principal_id|kind|payload)
  expires_at    TIMESTAMPTZ NOT NULL
  INDEX (principal_id, expires_at)
```

PG/SQLite parity is preserved because nothing in starter's queries
depends on engine-specific features. `LISTEN/NOTIFY` (PG) and polling
(SQLite) live behind a small `ChangeTail` trait in `starter-changelog`.

## Feature mapping (the "5 features, 1 primitive" claim, made concrete)

| Product feature | Mechanism |
|---|---|
| User audit log | `ChangeLog` filtered on `actor_kind = 'user'`. |
| AI-agent log | `ChangeLog` filtered on `actor_kind = 'agent'`. The `run_id` is opaque to the log; consumers with `starter-ai` join it themselves. Replay writes to the live store via `Reversible::apply_forward` — sandboxed / projected replay is a non-goal. |
| Undo / redo | Cursor over rows where `actor_id = me`, grouped by `group_id`. Undo = `apply_inverse` over the group in reverse `at` order. Redo = `apply_forward` forward. Stale rows raise `Error::Conflict` (via `resource_version` mismatch). |
| Duplicate | `clone_with(tx, src, {})` inside a `ChangeRecorder::transaction`. Records one or more `Create` changes under a single `group_id`. |
| Copy / paste | Copy persists `after` to `starter_clipboard` (HMAC-signed, principal-scoped, TTL'd). Paste = `clone_with(tx, src, overrides)`. Cross-session because the clipboard is server-side. |

## Non-goals (explicit)

- **No CRDTs, no operational transform, no multi-user concurrent editing.**
  Conflict resolution is last-write-wins; a stale undo surfaces an error
  the consumer renders.
- **No time-travel queries against arbitrary tables.** The log records
  what the consumer sent; it does not snapshot rows starter cannot see.
- **No global undo across actors.** Undo is per-`actor_id` by default. A
  consumer can choose to expose a broader scope, but starter's default
  is the safe one.
- **No automatic schema diffing.** The consumer decides what goes into
  `before` / `after`. Starter neither inspects nor validates payload
  shape beyond "valid JSON".
- **No UI components in this scope.** A future `@nube/starter-ui-undo`
  package may ship the keyboard hooks and history panel; it is out of
  scope for this document.
- **No sandboxed replay.** `Reversible::apply_forward` writes to the
  live store. Replaying an agent run into a snapshot or shadow database
  to inspect intent without side-effects is a real need, but it
  belongs behind a separate `Replayable::project_into(sandbox)` trait
  in a future doc — not on the back of `apply_forward`.

## Security & privacy

The changelog stores arbitrary `before` / `after` snapshots of consumer
data. That makes the audit and agent-log projections a confused-deputy
risk and a GDPR target. Two mechanisms, both in the seam:

- **Read-side ACL.** Alongside `Reversible`, the consumer registers a
  `ChangelogVisibility { fn may_read(principal: &Principal, ch: &Change) -> bool }`
  per resource kind. `starter-audit` and `starter-agent-log` MUST call
  it before returning a row. Default impl in `starter-changelog` is
  "deny unknown kinds" so a missing registration fails closed.
- **Per-resource tombstoning.** `ChangeRecorder::forget(resource: &ResourceRef)`
  nulls `before` / `after` / `patch` on every row matching the
  resource while preserving `(id, at, actor, op, group_id)` so replay
  integrity (row counts, ordering) survives. Consumers wire this into
  their right-to-erasure workflow; starter does not auto-prune.

Neither replaces table-level RBAC on `starter_changes`; both layer on
top of it.

## Open questions (resolve before first crate)

Resolved during peer review (see [Peer review log](#peer-review-log)):

- **Causation grouping API.** Decided: `ChangeRecorder::transaction(|tx| ...)`
  is the *only* path to record. No top-level `record()`. `group_id`
  is assigned by the recorder before the closure runs and shared by
  every row inside.
- **Optimistic concurrency.** Decided: `Change::resource_version:
  Option<u64>`. Stale undo surfaces `Error::Conflict` with the
  observed version in the message.
- **Composite resources.** Decided: `Reversible::clone_with` returns
  `Vec<ResourceRef>` and runs inside the caller's `ChangeTx` so all
  child rows land under one `group_id`.
- **Clipboard signing.** Decided: HMAC with a key fetched from
  `SecretStore` under the well-known name `starter.clipboard.hmac`.
  Rotated per-deploy.
- **`actor_model` column derivation.** Decided: the recorder always
  writes `actor_model` explicitly so the contract for every column
  lives in one place. The Postgres backend originally derived it
  with a `GENERATED` column; migration `0003_actor_model_explicit`
  drops that and re-adds a regular nullable `TEXT` column, backfilled
  from `actor_meta->>'model'`. The index on
  `(actor_kind, actor_model, at DESC)` is recreated unchanged.

Still open:

1. **Patch vs. full snapshot.** Start with full `before` / `after` JSON
   (simple, both engines, easy undo). Add `patch` as a size
   optimization once a real consumer hurts.
2. **Retention.** No automatic TTL pruning. `starter-changelog` ships
   a `prune` subcommand (owned by the core crate, not each backend)
   that the consumer schedules. Right-to-erasure is separate — see
   `ChangeRecorder::forget` in [Security & privacy](#security--privacy).

## Peer review log

- 2026-05-22 — Peer review surfaced 12 items. Doc updated for items
  1 (`Op::Custom(String)`), 2 (drop `json_patch::Patch` from spi
  surface, use `serde_json::Value`), 3 (reuse `starter_spi::Error`,
  no new `ReversibleError`), 4 (`resource_version`), 5 (`group_id`
  over self-referential `causation`), 6 (split clipboard storage
  shape into its own subsection), 7 (drop `starter-ai` arrow from
  `starter-agent-log`), 8+9 (new Security & privacy section), 10
  (`clone_with -> Vec<ResourceRef>`), 11 (sandboxed-replay non-goal),
  12 (`transaction` is the only recording path). Smaller items
  folded inline: reuse `starter_spi::authz::ResourceRef`, reuse
  `Principal::subject`, `#[async_trait]`, `actor_model` index,
  `SecretStore` key for clipboard HMAC, `ReversibleRegistry`
  registration shape.

## First concrete step

Land `starter-spi` additions behind a `changelog` feature, with
doc-comments only:

- New types: `Change`, `ChangeId`, `GroupId`, `TraceId`, `Actor`, `Op`,
  `ChangelogVisibility`.
- New traits: `ChangeRecorder`, `ChangeTx`, `Reversible`.
- Reuse without redefining: `starter_spi::authz::ResourceRef`,
  `starter_spi::auth::Principal`, `starter_spi::Error` /
  `starter_spi::Result`, `starter_spi::SecretStore` (referenced by
  `starter-clipboard` only — no new spi surface needed for it).

No backends, no readers, no dispatcher yet. That unblocks the parallel
design of `starter-changelog-sqlite` and `starter-changelog-postgres`
without committing to either's schema in `spi`.
