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
- **R3** — REST/gRPC/MCP/CLI handlers are thin: extract → call domain →
  `recorder.record(change)` → return. Undo/redo/paste endpoints dispatch
  through a registry; transport code knows nothing about resource kinds.
- **R5** — every crate listed below is opt-in. A consumer wanting only
  audit pulls `starter-changelog` + one backend + `starter-audit` and
  pays nothing for undo or clipboard.
- **R1** — one crate per concern. No `starter-history` god-crate.

## The seam (in `starter-spi`)

```rust
pub trait ChangeRecorder: Send + Sync {
    async fn record(&self, ch: Change) -> Result<ChangeId>;
}

pub struct Change {
    pub id: ChangeId,                       // ULID, monotonic
    pub actor: Actor,                       // User { principal_id }
                                            // | Agent { run_id, model }
                                            // | System
    pub resource: ResourceRef,              // { kind: &str, id: String }
    pub op: Op,                             // Create | Update | Delete | Custom(&'static str)
    pub before: Option<serde_json::Value>,  // for undo
    pub after:  Option<serde_json::Value>,  // for redo / duplicate / paste
    pub patch:  Option<json_patch::Patch>,  // optional, compact updates
    pub causation: Option<ChangeId>,        // groups multi-row ops into one undo step
    pub correlation: Option<TraceId>,
    pub at: DateTime<Utc>,
}

/// Implemented by the consumer, once per resource kind.
/// This is the ONLY extension point.
pub trait Reversible: Send + Sync {
    fn kind(&self) -> &'static str;
    async fn apply_inverse(&self, ch: &Change) -> Result<()>;   // undo
    async fn apply_forward(&self, ch: &Change) -> Result<()>;   // redo / paste
    async fn clone_with(
        &self,
        src: &ResourceRef,
        overrides: serde_json::Value,
    ) -> Result<ResourceRef>;                                    // duplicate / paste-as-new
}
```

That is the entire surface. New resource kind = one `Reversible` impl,
registered at server build time. No fork of any starter crate.

## Crates

Each does one job (R1) and is independently optional (R5).

| Crate | Job | Depends on |
|---|---|---|
| `starter-changelog` | `ChangeRecorder` trait, `Change` type, `ChangeLog` query API (filter by actor / resource / time / causation). No SQL. | `starter-spi` |
| `starter-changelog-sqlite` | One table `starter_changes`, namespaced migration, `ChangeRecorder` impl. | `starter-changelog`, `starter-store-sqlite` |
| `starter-changelog-postgres` | Same shape; `jsonb` payload, optional `LISTEN/NOTIFY` tail. | `starter-changelog`, `starter-store-postgres` |
| `starter-audit` | Read-only projection over `ChangeLog` filtered to `Actor::User`. REST + CLI. | `starter-changelog`, `starter-server` |
| `starter-agent-log` | Read-only projection filtered to `Actor::Agent`. Joins to `starter-ai` run-id for session replay. | `starter-changelog`, `starter-ai` |
| `starter-undo` | Per-actor undo/redo cursor over `starter_changes` grouped by `causation`. Dispatches through the `Reversible` registry. | `starter-changelog` |
| `starter-clipboard` | Copy = persist `after` of a `ResourceRef` into `starter_clipboard` (signed, TTL'd, scoped to principal). Paste = `Reversible::clone_with`. Duplicate = copy+paste in one call. | `starter-changelog` |

No crate above knows what a "note" or a "dashboard" is. The consumer's
domain code is where kinds are defined and `Reversible` is implemented.

## Storage shape (sketch, same logical schema both engines)

```
starter_changes
  id            TEXT/UUID PRIMARY KEY        -- ULID
  at            TIMESTAMPTZ NOT NULL
  actor_kind    TEXT NOT NULL                -- 'user' | 'agent' | 'system'
  actor_id      TEXT                         -- principal_id or run_id
  actor_meta    JSON/JSONB                   -- e.g. { model: "claude-..." }
  resource_kind TEXT NOT NULL
  resource_id   TEXT NOT NULL
  op            TEXT NOT NULL
  before        JSON/JSONB
  after         JSON/JSONB
  patch         JSON/JSONB
  causation     TEXT REFERENCES starter_changes(id)
  correlation   TEXT
  INDEX (resource_kind, resource_id, at DESC)
  INDEX (actor_kind, actor_id, at DESC)
  INDEX (causation)

starter_clipboard
  id            TEXT PRIMARY KEY
  principal_id  TEXT NOT NULL
  resource_kind TEXT NOT NULL
  payload       JSON/JSONB NOT NULL          -- the `after` snapshot
  signature     BLOB/BYTEA NOT NULL
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
| AI-agent log | `ChangeLog` filtered on `actor_kind = 'agent'`, optionally joined to `starter-ai` run-id. Replay = iterate forward, call `Reversible::apply_forward`. |
| Undo / redo | Cursor over rows where `actor_id = me`, grouped by `causation`. Undo = `apply_inverse` over the group in reverse. Redo = `apply_forward` forward. |
| Duplicate | `clone_with(src, {})`. Records a `Create` change with `causation = None`. |
| Copy / paste | Copy persists `after` to `starter_clipboard`. Paste = `clone_with(src, overrides)`. Cross-session because the clipboard is server-side, per-principal. |

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

## Open questions (resolve before first crate)

1. **Patch vs. full snapshot.** Start with full `before` / `after` JSON
   (simple, both engines, easy undo). Add `patch` later as a size
   optimization. Decision: defer until a real consumer hurts.
2. **Retention.** Per-resource TTL? Per-actor cap? Initial answer: no
   automatic pruning; ship a `starter-changelog prune` CLI command and
   let the consumer policy it.
3. **Signing the clipboard payload.** HMAC with a key from
   `SecretStore`? Or rely solely on principal-scoped row access?
   Leaning HMAC so a leaked row can't be replayed cross-principal.
4. **Causation grouping API.** Likely a `recorder.transaction(|tx| { ... })`
   helper that assigns one `causation` id to every `record()` call inside.
   Confirms once the first consumer wires it.

## First concrete step

Land `starter-spi` additions (`Change`, `Actor`, `Op`, `ResourceRef`,
`ChangeRecorder`, `Reversible`) behind a `changelog` feature, with
doc-comments only. No backends, no readers, no dispatcher yet. That
unblocks the parallel design of `starter-changelog-sqlite` and
`starter-changelog-postgres` without committing to either's schema in
`spi`.
