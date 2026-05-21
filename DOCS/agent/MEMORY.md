# Agent Memory & Sessions — Scope (proposed)

Status: **proposed, awaiting approval** · Owner: starter-flow / starter-agent
· Companions: [SCOPE.md](./SCOPE.md) (R-agent-2), [SKILLS.md](./SKILLS.md)

> This doc owns the **persistence of agent conversation state** across
> turns, page loads, process restarts, and host migration. It does not
> own the LLM loop ([SCOPE.md R-agent-1](./SCOPE.md)), the LLM seam
> ([SCOPE.md R2](./SCOPE.md)), or skill bundles ([SCOPE.md R4](./SCOPE.md)).

---

## 1. One-line summary

A single `SessionStore` trait in `starter-flow-spi`, with `SQLite` and
`Postgres` impls, that persists every agent turn and every named
artifact a surface produces — separated from a per-surface **replay
policy** that decides what (if anything) is fed back into the model on
the next turn.

## 2. Why this exists

Today the `flow-agent` `/api/builder/stream` route is stateless: every
prompt is a brand-new CLI/REST call, the model sees no prior turns, and
the page builder's tree is reconstructed from scratch each time. This
is fine for a demo, wrong for product.

The naive fix — "replay every prior turn into the model on every call" —
is the worst of both worlds: tokens grow O(N), page-open cost grows
O(N), and the model is forced to re-derive state it already produced.

Two observations untangle this:

1. **Storage and replay are separate decisions.** Always store
   everything (audit, debug, undo, analytics). Decide separately what
   the model sees on the next turn.
2. **Different surfaces want different replay.** A page builder's state
   IS the current tree — the conversation is incidental. A chat
   assistant's state IS the conversation. A debugger view wants every
   turn verbatim. One replay strategy can't serve all three.

## 3. Hard rules (load-bearing)

### M1 — One `SessionStore` trait; two backends; pick at composition time

`starter-flow-spi::SessionStore` is the single seam. `starter-store-sqlite`
and `starter-store-postgres` provide impls. A binary picks one in
`main.rs` via `Engine::builder().with_store(...)` exactly like the
existing `RunStore` / `flow` feature does. No second persistence layer
for agents — they reuse the engine's store. (Matches
[SCOPE.md R-agent-2](./SCOPE.md).)

### M2 — Three tables, same schema in both backends

```sql
sessions (
  id           text primary key,             -- ULID, lexicographically time-sorted
  kind         text not null,                -- "page-builder", "chat", ...
  owner        text not null,                -- principal id; "system" for unowned
  created_at   timestamptz not null,
  updated_at   timestamptz not null,
  metadata     jsonb not null default '{}'   -- reserved keys: provider, model, flow_id
);

session_turns (
  session_id      text not null references sessions(id) on delete cascade,
  seq             integer not null,           -- monotonic per session, store-assigned
  role            text not null,              -- "user" | "assistant" | "tool"
  content         jsonb not null,             -- normalised turn payload, schema_version on row
  schema_version  integer not null default 1, -- bump when Turn.content shape changes
  content_bytes   integer not null,           -- size of `content` after serialisation; enforced cap (M11)
  tokens_in       integer,                    -- nullable; CLI runners often don't report
  tokens_out      integer,                    -- nullable; same
  created_at      timestamptz not null,
  primary key (session_id, seq)
);

session_artifacts (
  session_id      text not null references sessions(id) on delete cascade,
  key             text not null,              -- "tree", "draft", "__summary", ...
  version         integer not null,           -- monotonic per (session, key), store-assigned
  parent_version  integer,                    -- the version this one was edited from; null for v1
  value           jsonb not null,
  value_bytes     integer not null,           -- enforced cap (M8)
  produced_by_seq integer,                    -- the turn seq that produced this; null for direct writes
  updated_at      timestamptz not null,
  primary key (session_id, key, version),
  foreign key (session_id, produced_by_seq) references session_turns(session_id, seq)
);
```

`parent_version` enables lineage without committing to a branching UX —
linear undo writes `parent_version = current - 1`; a surface that wants
branching writes `parent_version = <whatever the user undid to>`. The
store records the graph; the surface decides what it means.

`__summary` is a reserved artifact key (M3 strategy
`summary_plus_tail`); see M3 below.

`session_turns` is the audit log. `session_artifacts` is the snapshot
log — every saved version of a named piece of state. The page builder's
tree lives here as `key = 'tree'`; chat surfaces won't use this table at
all.

Indices: `(session_id, seq desc)` on turns, `(session_id, key, version
desc)` on artifacts. Both are point-lookups in practice.

SQLite uses `text` for timestamps (ISO-8601) per the existing
`starter-store-sqlite` convention; Postgres uses native `timestamptz`.
Migrations live alongside the existing flow-engine migrations in each
backend crate. **No** schema divergence between backends beyond column
types — the trait surface is identical.

### M3 — Replay policy is per-surface, declared on the `ai-agent` node

The `ai-agent` node config slot already carries `session_policy`
([SCOPE.md R-agent-2](./SCOPE.md)). Extend it with a `replay` sub-field:

```yaml
- id: builder
  kind: ai-agent
  config:
    session_policy: continue           # fresh | continue | long-lived
    replay:
      strategy: snapshot               # snapshot | full | summary_plus_tail | none
      artifact_keys: [tree, theme]     # required when strategy = snapshot; list, not scalar
      tail_k: 6                        # required when strategy = summary_plus_tail
      summarise_every_k_turns: 10      # required when strategy = summary_plus_tail
      summariser: cheap-haiku          # optional; default = built-in route
```

Four strategies, exhaustively:

| Strategy             | What the model sees on turn N                                  | Cost     | Right for                                  |
|----------------------|----------------------------------------------------------------|----------|--------------------------------------------|
| `none`               | New prompt only                                                | O(1)     | One-shot tools, today's demo               |
| `snapshot`           | Latest values of every key in `artifact_keys` + new prompt     | O(1)     | Page builder (tree IS the state)            |
| `summary_plus_tail`  | `latest_artifact('__summary')` + last `tail_k` turns verbatim  | O(1)     | Chat assistants, long-running agents        |
| `full`               | Every prior turn verbatim                                      | O(N)     | Debugger view, short tool conversations     |

**`summary_plus_tail` summariser lifecycle** (peer review #4 — must be
spelled out or the "O(1)" claim is a lie):

- The summary is stored as artifact `key = '__summary'`, versioned like
  any other artifact.
- After every `summarise_every_k_turns` appended turns, the engine
  enqueues a background summariser call: read turns since
  `__summary`'s `produced_by_seq`, plus the prior summary, produce a
  new `__summary` artifact via the configured `summariser` route.
- The replay strategy reads `latest_artifact('__summary')` + the last
  `tail_k` turns. If `__summary` doesn't exist yet (early in a
  session), it's omitted; cost is bounded by `tail_k`.
- Summariser failure is non-fatal: the next turn uses the stale
  summary and tries again. Logged, not raised.

The store always records everything. The strategy decides the **read
path** into the model. Page reload of an artifact (e.g. the page
builder fetching the latest tree to render) goes through a separate
`SessionStore::latest_artifact(session_id, key)` call and **never**
invokes the model.

### M4 — Surfaces fetch artifacts directly; only the model call uses replay

Two distinct read paths:

1. **`GET /api/sessions/:id/artifacts/:key`** — frontend on page load.
   Returns the latest artifact JSON. **Zero model tokens.** This is how
   the page builder rehydrates the canvas without spending budget.
2. **`POST /api/builder/stream` (or any `ai-agent` invocation)** — when
   the user actually prompts. The engine reads the session, applies the
   declared replay strategy, builds the model input, calls `AiRunner`.

These are not the same code path. Treating "load the page" and "ask the
model" as the same operation is the bug we are pre-empting.

### M5 — Append-only turns; artifacts are versioned, not overwritten

`session_turns` is append-only. `session_artifacts` writes a new row
per save (monotonic `version`); the "latest" read is `order by version
desc limit 1`. **Reasons:**

- Undo / version history fall out for free.
- Debug surfaces can replay history without joining against a CDC log.
- Concurrent writers (rare — two tabs editing the same page) don't
  lose work; the store assigns versions atomically (see below) and
  conflict resolution above that is a surface concern.

**Concurrency contract (peer review #1 — pick one and put it in the
trait):** version assignment is **store-side under a transaction**.
SQLite uses `BEGIN IMMEDIATE`; Postgres uses `SELECT max(version) ...
FOR UPDATE` against the `(session_id, key)` row group, then insert.
Callers never compute `next_version` client-side; the trait returns
the assigned version. Optimistic concurrency is **not** the default;
surfaces that want it pass `expected_prev_version: Option<u32>` and
get a typed `Conflict { current: u32 }` error on mismatch — see M11
trait surface.

Retention is a separate concern (M9).

### M6 — Provider-native session ids are an optimisation, not the substrate

Claude CLI's `--continue` and Copilot CLI's session cache are
**caches**, not the source of truth. The runner may opportunistically
pass `session_id` through to the provider when the strategy is
`full` or `summary_plus_tail` on a CLI provider, to save tokens. If the
provider's cache has evicted the session, the runner reconstructs the
input from `SessionStore` and resumes. The model never sees a gap;
the surface never sees a provider switch.

This means: a session that started on `Provider::Claude` (CLI) can be
resumed on `Provider::Anthropic` (REST) without data loss. The store is
the substrate; the CLI cache is a perf hint.

### M7 — Session ids are surface-owned ULIDs, not provider ids

The surface generates a `session_id` (ULID, in `sessions.id`) before
the first call. It never leaks the provider's internal session token
to clients. Reasons: provider tokens are opaque, change format,
sometimes carry PII, and tie us to one runner. ULIDs sort by time and
are URL-safe.

### M8 — Snapshot strategy MUST cap artifact size in the prompt

When `strategy: snapshot`, each artifact's JSON is inlined into the
system prompt under a fixed delimiter. **Hard caps:**

- **Per-artifact: 32 KB** serialised. A single oversized artifact is
  truncated with a documented tail marker; an `error` frame surfaces
  to the client.
- **Aggregate (sum across `artifact_keys`): 96 KB** serialised. If the
  sum exceeds the cap, artifacts are dropped in reverse declaration
  order until under the limit, with the same tail marker on the last
  partially-included one. An `error` frame surfaces the drop list.

**Reason:** the page builder's tree can balloon; without a cap, one
big save bricks every subsequent turn for that session. Multi-artifact
snapshot makes the aggregate cap necessary — the model only has one
context window. `full` and `summary_plus_tail` have their own bounds
(token-based, applied by the summariser).

### M9 — Retention is opt-in per session kind, not a global TTL

The `sessions` table grows forever by default. A binary opts into
retention via `Engine::builder().with_session_retention(kind, policy)`:

- `keep_forever` (default)
- `delete_after { duration }` — hard delete via `on delete cascade`
- `delete_turns_after { duration, keep_latest_artifact: bool }` — prune
  conversation, keep the artifact (right for the page builder: forget
  *how* the page was built after 90 days, keep the page)

Retention runs as a scheduled job inside the engine's existing
maintenance loop. **Reason:** different surfaces have different
compliance and storage needs; a global TTL would either be too short
for product use or too long for GDPR/cost.

### M10 — Cancellation, crashes, and streaming never corrupt the session

A turn is written **after** the runner finishes (success or error). A
partial / cancelled turn is not persisted as `assistant` — it's
optionally recorded as a `tool` row with `role: "system"` and a
`cancelled: true` marker in `content`, **only if** the surface opts in.
Default: drop. **Reason:** replaying a cancelled turn confuses the
model into thinking it produced output it didn't.

**Streaming surfaces (peer review #8):** intermediate SSE frames
(`patch`, `full-render` during streaming) are **never** written as
artifacts. Only the terminal state on `status: done` is committed.
Reload mid-stream returns the prior committed artifact, not a
half-rendered one. The frontend's transient view-model is its own
problem; the store's view is "the last successful turn."

**Transactionality is at the trait level, not the SQL level (peer
review #5).** Per-turn writes — the turn row plus any artifacts it
produced — go through a single trait method
`append_turn_with_artifacts(turn, &[(key, value, parent_version?)]) ->
Result<TurnReceipt>` that returns the assigned `seq` and the assigned
artifact versions. Implementations wrap the whole thing in one
transaction (SQLite `BEGIN IMMEDIATE`, Postgres default isolation with
explicit `BEGIN`). Callers cannot construct a partial state. See M11.

### M11 — Trait surface honours M5, M8, M10 mechanically

The trait is shaped so that no implementation can violate the
concurrency, transactionality, or size-cap rules. See §4.1 for the
full signature. Three load-bearing shapes:

- `append_turn_with_artifacts(...)` is the **only** way to write a
  turn; it accepts the turn plus its artifacts in one call and assigns
  all versions store-side. No separate `put_artifact` from inside an
  agent loop.
- `put_artifact_direct(...)` exists for surface-initiated writes
  (e.g. a manual rename) and takes `expected_prev_version: Option<u32>`
  for optimistic concurrency. Returns `Err(Conflict { current })` on
  mismatch.
- Size caps (M8 per-artifact, M12 per-turn) are enforced in the trait's
  default validation layer, not per-backend. Oversize writes return a
  typed error, not a truncated row.

### M12 — Turn payload is capped; large content moves to artifacts

`session_turns.content` carries the normalised turn (role, message
text, tool calls and results). **Hard cap: 64 KB serialised per turn.**
Larger payloads (e.g. a multi-MB tool result) must be written as an
artifact and referenced from the turn by `{ "$artifact": { "key":
"...", "version": N } }`. The replay layer dereferences these when
materialising the turn for the model.

**Reason (peer review #7):** without a cap, a chatty tool loop produces
multi-MB rows that wreck pagination, replay performance, and audit
queries. With a cap, the turn row stays index-friendly and the
artifact table absorbs whatever needs absorbing — which is exactly what
the artifact table is for.

### M13 — `/api/builder/stream` without `session_id` stays ephemeral, forever

Backwards compatibility for the existing stateless contract (peer
review #9): when `session_id` is omitted from the request, the route
creates an ephemeral in-memory session that is **never persisted** —
no row in `sessions`, no turn, no artifact. Behaviour is identical to
today's demo. The `session_id` is also not returned in the response.

No deprecation window. The opt-in is positive ("pass `session_id` to
get persistence"), not negative ("pass a flag to keep the old
behaviour"). Existing clients neither break nor silently accumulate
rows.

---

## 4. Surfaces

### 4.1 New trait (in `starter-flow-spi`)

```rust
#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    // ----- lifecycle -----
    async fn create(&self, kind: &str, owner: &str /* "system" if unowned */)
        -> Result<SessionId>;
    async fn get(&self, id: &SessionId) -> Result<Option<Session>>;
    async fn delete(&self, id: &SessionId) -> Result<()>;

    // ----- writes (M10, M11) -----
    /// Single transactional write of a turn plus any artifacts it produced.
    /// Store assigns `seq` and every artifact `version`. The only write path
    /// used by the agent loop.
    async fn append_turn_with_artifacts(
        &self,
        id: &SessionId,
        turn: TurnInput,                                  // role, content, tokens
        artifacts: &[ArtifactWrite],                      // key, value, parent_version?
    ) -> Result<TurnReceipt>;                             // assigned seq + versions

    /// Surface-initiated artifact write (e.g. manual rename, save-as).
    /// Optimistic concurrency via `expected_prev_version`.
    async fn put_artifact_direct(
        &self,
        id: &SessionId,
        key: &str,
        value: serde_json::Value,
        expected_prev_version: Option<u32>,               // None = unconditional
    ) -> Result<u32, PutArtifactError>;                   // Conflict { current: u32 } variant

    // ----- reads -----
    async fn list_turns(
        &self, id: &SessionId, since_seq: Option<u32>, limit: Option<usize>,
    ) -> Result<Vec<Turn>>;
    async fn latest_artifact(&self, id: &SessionId, key: &str) -> Result<Option<Artifact>>;
    async fn artifact_at(&self, id: &SessionId, key: &str, version: u32) -> Result<Option<Artifact>>;
    async fn list_artifact_versions(&self, id: &SessionId, key: &str)
        -> Result<Vec<ArtifactMeta>>;                     // includes parent_version
}

pub enum PutArtifactError {
    Conflict { current: u32 },
    TooLarge { bytes: usize, cap: usize },                // M8
    Backend(anyhow::Error),
}
```

Notes:
- No bare `append_turn` / `put_artifact`. The two write methods above
  are the only ways to mutate, and both assign versions store-side
  (M5 concurrency contract).
- `TurnReceipt` carries the assigned `seq` plus a `Vec<u32>` of
  artifact versions in declaration order, so the caller can echo
  them back to the client without a second read.
- `since_seq` on `list_turns` is the read path the `full` and
  `summary_plus_tail` replay strategies use; bounded by `limit`.

### 4.2 New / edited crates

| Path                                                | Change | Purpose                                                    |
|-----------------------------------------------------|--------|------------------------------------------------------------|
| `crates/starter-flow-spi/src/session.rs`            | new    | `SessionStore` trait + types                                |
| `crates/starter-store-sqlite/src/session.rs`        | new    | SQLite impl + migrations                                    |
| `crates/starter-store-postgres/`                    | new    | Postgres impl (may already exist for flow runs; extend)     |
| `crates/starter-flow/src/replay.rs`                 | new    | Replay strategies (none/snapshot/full/summary_plus_tail)    |
| `crates/starter-flow-node-loop/src/lib.rs`          | edit   | Read replay config, call store, build model input            |
| `examples/flow-agent/src/builder_stream.rs`         | edit   | Accept `session_id?`, persist turn + tree artifact            |
| `examples/flow-agent/src/rest.rs`                   | edit   | Add `GET /api/sessions/:id/artifacts/:key`                  |
| `examples/flow-agent/frontend/src/pages/PageBuilder.tsx` | edit | On mount, fetch latest tree; send `session_id` per call    |

### 4.3 Wire contract changes

**Request body extension** (backwards-compatible — fields optional):

```json
{
  "prompt": "add a CPU chart",
  "provider": "claude",
  "session_id": "01HZX...",         // omit to start fresh
  "include_artifact": "tree"        // omit if surface manages its own state
}
```

**New endpoint:**

```
GET /api/sessions/:id/artifacts/:key
→ 200 { "key": "tree", "version": 7, "value": { ... }, "updated_at": "..." }
→ 404 if no such artifact
```

## 5. Page builder flow, end-to-end (the worked example)

1. User clicks **New page** → frontend `POST /api/sessions { kind: "page-builder" }` → gets `session_id`. Stored in URL as `/pages/new?sid=01HZX...`.
2. User prompts "iot dashboard" → frontend `POST /api/builder/stream { prompt, session_id, include_artifact: "tree" }`. Backend: no prior tree, so `snapshot` replay yields just the prompt; runner produces tree; backend `append_turn` + `put_artifact("tree", v1)` transactionally; SSE streams `full-render`.
3. User saves → tree persisted (already done in step 2 — no extra call).
4. User closes tab. Comes back tomorrow, visits `/pages/01HZX...` → frontend `GET /api/sessions/01HZX.../artifacts/tree` → renders canvas. **Zero model tokens spent on page load.**
5. User prompts "add a CPU chart" → frontend `POST /api/builder/stream { prompt, session_id, include_artifact: "tree" }`. Backend: `snapshot` replay inlines latest tree into system prompt + new prompt; runner produces modified tree; `append_turn` + `put_artifact("tree", v2)`; SSE streams `full-render`. Model sees the tree, not the prior conversation — but the conversation is on disk for audit/undo.
6. User clicks **Undo** → frontend `GET /api/sessions/01HZX.../artifacts/tree/versions` → picks v1 → frontend renders v1. **Still zero model tokens.**

The model is invoked **only** when the user prompts. Loading, viewing, and undoing are pure storage operations.

## 6. Phasing

### Phase M-A — `SessionStore` trait + SQLite impl + migrations
- Trait in `starter-flow-spi`.
- SQLite impl + migration.
- Unit tests: create, append, latest_artifact, version monotonicity, cascade delete.
- **Not yet wired into any route** — pure plumbing.

### Phase M-B — Postgres impl
- Mirror SQLite. Same test suite, same fixtures.
- CI matrix: both backends pass identical integration tests against the trait.

### Phase M-C — Replay strategies + `ai-agent` node integration
- `replay.rs` with all four strategies.
- `starter-flow-node-loop` reads config, builds model input via strategy.
- Cap enforcement (M8).

### Phase M-D — Page builder cutover
- `/api/builder/stream` accepts `session_id` + `include_artifact`.
- `GET /api/sessions/:id/artifacts/:key` endpoint.
- Frontend rehydrates from artifact on page open.
- E2E: open new page, prompt, save, close tab, reopen, see canvas, prompt again, see incremental edit.

### Phase M-E — Retention + maintenance
- `with_session_retention` builder method.
- Scheduled prune job inside the engine maintenance loop.
- Opt-in per surface; nothing deleted by default.

---

## 7. Non-goals

- **Vector / embedding memory.** Not in this doc. "Remember the user's
  preferences across unrelated sessions" is a separate problem solved
  by a different store (likely an extension).
- **Cross-session memory sharing.** A session is the unit of context.
  Surfaces wanting shared memory build it on top of artifacts with
  shared keys (separate doc when needed).
- **Provider-side memory features** (Anthropic's beta memory tool, etc.).
  Treated as caches per M6 if exposed at all; never the substrate.
- **Real-time collaboration** on a single session. Append-only turns
  + versioned artifacts make this *possible*, but the conflict UX is
  out of scope.

## 8. Open questions

1. **Summariser provider.** `summary_plus_tail` needs a cheap model
   call. Default to the same provider as the main agent, or a fixed
   "cheap-haiku" route? Probably the latter, gated behind a binary
   config.
2. **Artifact diffing.** Storing every tree version in full is simple
   but wasteful for tiny edits. Add a JSON-patch column later? Not in
   v1.

## 9. Decisions made

- **Storage and replay are separate concerns.** M3 + M4. The single
  most important decision in this doc.
- **One `SessionStore` trait, two backends, identical surface.** M1 +
  M2. Reuses the engine's store; no second persistence stack.
- **`SessionStore` is its own trait alongside `RunStore`, in the same
  backend crate** (peer review #10). `Engine::builder()` gains
  `with_session_store(...)`; `starter-store-sqlite` and
  `starter-store-postgres` each provide both impls. One crate per
  backend, not two.
- **Append-only turns + versioned artifacts with `parent_version`
  lineage** (peer review #2). M5. Linear undo and branching are both
  expressible; branching UX is the surface's call, not the store's.
- **Surface-owned ULIDs, not provider tokens.** M7. Decouples
  persistence from runner choice. ULIDs sort lexicographically by
  time, so `order by id` ≈ `order by created_at` — cheap pagination.
- **Page builder uses `snapshot`, not `full` replay.** §5. The tree IS
  the state; conversation history is for audit.
- **`sessions.owner` is required, not nullable** (peer review #6).
  Unowned sessions use `owner = "system"`. Avoids the
  grandfather-vs-break dilemma if auth is added later. When a
  `Principal` is present on the request, the store enforces
  `session.owner == principal` on read; otherwise reads with `"system"`
  owner are unrestricted.
- **`schema_version` on every turn row** (peer review minor §4 / former
  Q4). Readers handle every version they know about; writers always
  write current. Cheap to add now, painful to add later.
- **`metadata jsonb` on sessions has documented reserved keys:**
  `provider`, `model`, `flow_id`, `client_kind`. Surfaces use these
  for free; other keys are unrestricted but should be namespaced.
- **`tokens_in` / `tokens_out` are nullable on purpose.** CLI runners
  often don't report. Absence is expected, not a bug — analytics that
  group by token usage must handle null explicitly.

## 10. Bottom line

**Store everything, replay selectively.** One trait, two backends,
four replay strategies, a separate read path for surfaces that just
want to render the latest artifact without spending model tokens. The
page builder is the first consumer; chat surfaces, debuggers, and
extension-contributed agents all use the same substrate with different
replay strategies declared in their flow YAML.

No second persistence layer. No global "memory". No provider-locked
session ids. The model sees what each surface decides it should see,
and the store keeps a complete, queryable record either way.
