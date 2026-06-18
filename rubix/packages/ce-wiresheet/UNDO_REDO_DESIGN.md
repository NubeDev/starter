# Engine-side undo / redo

> **Status — SHIPPED (verified in the live `openapi.yaml`).** The engine moved
> past the "strictly per-request" grouping below to a **gesture id**, and
> expanded the journal beyond structural ops. What's live now:
>
> - **`X-Gesture-Id`** (int32) header on every writer endpoint. All writes sharing
>   one non-zero id are grouped into **one atomic undo entry** — both a streamed
>   drag (many position writes → one undo) and a compound gesture across endpoints
>   (Group = add folder + reparent + facets → one undo). Omit → engine's short
>   time-window coalescing.
> - **Expanded `ChangeOp`**: `updateName`, `setValue`, `updateMetadata`, **`group`**
>   joined the structural/override set — so **rename, value/config/`__facets`
>   edits, and position/size drags are now undoable** (the deferred "phase 2"
>   property journaling landed). A `group` entry's `componentUids`/`edgeUids` are
>   the union of its members and it undoes/redoes atomically.
> - `/undo`, `/redo`, `/changelog`, `X-Actor-Id`, and per-item `changeId`/`actorId`
>   on write responses + the WS push (all confirmed earlier).
>
> **Client (`@nube/ce-wiresheet`) — DONE.** `lib/rest.ts` sends `X-Actor-Id`
> (per-tab, from the session id) + `X-Gesture-Id` (`setRestGestureId`/`newGestureId`/
> `withGesture`); `Cmd-Z` / `Cmd-Shift-Z` / `Ctrl-Y`. Group, paste, and drag each
> stamp one gesture id so they undo atomically. **Still blocked:** the reparent
> `std::bad_alloc` (engine) gates Group actually completing.
>
> The text below is the original engine planning doc, kept for the rationale.

---

Local planning doc (`docs/plans/` is gitignored). Design only — no code yet.

Move undo/redo from the client (REST FE) into the engine. A writer API request carries an
**actor id**; the engine records the change plus its **inverse**, echoes the id on the resulting
event, and exposes `undo(actorId)` / `redo(actorId)` that replay the inverse. The engine is the
right home because it already owns the inverse primitives — most importantly **same-UID restore**,
which a client cannot reproduce — and is the only place multi-client undo can stay consistent.

## Decisions (settled)

| Question | Decision |
|---|---|
| Where | Engine-side, persisted. |
| Actor key | `int32 actorId`, extension id in the high bits → globally unique. An install runs REST **or** NATS, not both, so `actorId` alone is a safe stack key (no `(origin, actorId)` tuple needed). |
| What's undoable | **Only SDK/engine writer-API ops.** Never evaluation-driven output changes. |
| Conflict on undo | **Reject if not clean** — per-op precondition, reason returned + emitted as event. |
| Horizon | Bounded by the soft-delete retention window; undo-of-delete dies when the UID is purged. |
| Persistence | **SQLite** — new table + migration; survives restart (soft-deleted UIDs already reserved at boot). |
| Grouping | **Strictly per-request.** One writer RPC = one undo entry. |

### Consequence of per-request grouping

A multi-item gesture ("paste 5 wired nodes") is one clean undo unit **only if the FE issues it as a
single batch RPC** (`addComponentsAndEdges` / `copyComponents`). Those are already one RPC = one
entry, so batch ops give correct gesture-undo for free. If REST instead loops single calls, the user
gets N separate undos. **Action for the REST side: map a multi-item gesture to the batch API.**

## Open fork — `actorId` vs `changeId` (recommendation below)

Two distinct ids, recommend carrying both:

- **`actorId`** — stable per user (`user-42`), high bits = ext id. Scopes the undo stack; stamped on
  every event. Drives the plain Undo button: `undo(actorId)` pops the actor's most-recent applied
  change.
- **`changeId`** — engine-assigned, unique per recorded change (DB rowid / monotonic seq). Identifies
  one entry for targeted `undo(actorId, changeId)` and is echoed in the event so the FE correlates
  its own change.

Common path is `undo(actorId)` / `redo(actorId)`. `changeId` is optional/targeted. *(Flip this if you
want a single client-supplied id that serves both.)*

## What the engine already has (reuse, don't rebuild)

- `restoreComponentByUID` / `restoreEdgeByUID` / `restoreItems` — already labelled "for undo" in
  `app.capnp:25`. Restore brings things back **at the same UID** (`reserveSoftDeletedAt`). Keystone.
- `DeletedItems` / `BatchAddResult` already return the affected UIDs (parent-first) — the raw material
  for an inverse op.
- `FlowGraph::withEvalHold` — atomic multi-write transaction; the graph never evaluates a half-wired
  state. Undo replay runs inside it.
- `softDeletePurgeLoop` purge callback — the GC hook for expiring undo entries (see Horizon).
- Event pool already stamps `seq_id` + `ts_ns`; add the actor/change ids alongside.

## Inverse table

| Forward op | Inverse | Primitive status |
|---|---|---|
| addComponent | remove (soft-delete) | ✅ exists |
| removeComponent | restoreComponentByUID (same UID) | ✅ exists |
| addEdge | removeEdge | ✅ exists |
| removeEdge | restoreEdgeByUID | ✅ exists |
| moveComponent | move back to old parent + old name | ✅ engine knows old parent/name |
| addComponentsAndEdges (batch) | removeComponentsAndEdges over created UIDs | ✅ exists |
| copyComponents | removeComponentsAndEdges over created UIDs | ✅ exists |
| set override | clear override | ✅ exists |
| clear override | re-set override (value + remaining duration) | ✅ exists |
| **input value write** | restore previous value | ⚠️ deferred — see Scope |

## Scope

**v1:** structural ops (add/remove/move component, add/remove edge, the two batch ops) +
override set/clear. These have clean inverses and clean preconditions.

**Deferred (phase 2):** raw input-value reverts. "Previous value" is ambiguous in a live dataflow
graph (outputs recompute every cycle; inputs may be cascade-driven or overridden). Either model
user value sets as overrides (already undoable) or snapshot the pre-write resolved value behind an
explicit precondition. Not in v1.

## Preconditions (the "reject if not clean" rule, per op)

Checked before replaying the inverse; failure → reject with reason (response + event), entry stays
`APPLIED` (or `UNDONE` for redo).

- undo `addComponent` (→ delete): component still `OCCUPIED` and has gained **no** children/edges
  from another actor since → else reject.
- undo `removeComponent` (→ restore): UID still `RESTORABLE` (within window) **and** parent still
  exists → else reject (`"expired"` / `"parent gone"`).
- undo `addEdge` (→ removeEdge): edge still exists, endpoints unchanged.
- undo `removeEdge` (→ restore): edge UID still `RESTORABLE`; both endpoint components live.
- undo `moveComponent` (→ move back): component still sits at the parent we moved it to (nobody
  re-moved it).
- undo override set/clear: current override state matches what this entry produced.

## Persistence

New table (via `MigrationManager`):

```
change_log(
  change_id     INTEGER PRIMARY KEY,   -- engine seq / rowid
  actor_id      INTEGER NOT NULL,      -- ext-id high bits + user
  op_type       INTEGER NOT NULL,
  forward_blob  BLOB,                  -- serialized request (for redo / audit)
  inverse_blob  BLOB,                  -- serialized inverse op + precondition
  state         INTEGER NOT NULL,      -- APPLIED | UNDONE | EXPIRED
  seq           INTEGER NOT NULL,      -- per-actor ordering
  created_at    INTEGER NOT NULL
)
```

- **Reuse the existing single DB writer thread** — do *not* add a thread. The single-writer-to-SQLite
  model is the whole point of that thread; a second writer would break it. Add **one new STORE-grade
  job type** + the table above, nothing more. STORE-grade = durable promptly (a crash right after the
  op must still leave it undoable); the current writer already batches STORE (~10ms) / UPDATE (60s).
  `flushDb` on `IEngineWriter` already forces a drain when needed.
- **Commit ordering: log row commits *after* the mutation.** Mutation-then-no-log → degrades to
  "not undoable" (safe). Log-then-no-mutation → the precondition check catches it on undo (safe).
  Never reverse the order.
- **Boot replay:** load `APPLIED`/`UNDONE` rows, rebuild per-actor stacks. Inverses that say "restore
  UID X" stay valid because `reserveSoftDeletedAt` already re-parks soft-deleted UIDs at boot.

## GC — coupled to purge, not a timer

An entry whose inverse is "restore UID X" is valid only while X is `RESTORABLE`. When
`softDeletePurgeLoop` (or the force `purgeSoftDeleted` action) purges X, mark the referencing
change-log entry `EXPIRED` **in the same purge callback** that already fires per-uid. Otherwise undo
entries point at purged UIDs. This is the concrete shape of the Horizon bound.

## Redo

Per-actor redo stack. `undo` moves an entry `APPLIED → UNDONE` and pushes it onto redo;
`redo(actorId)` re-applies the **forward** op (`forward_blob`) and moves it back to `APPLIED`. Any
**new forward op by that actor invalidates its redo stack** (standard). Redo has the same
precondition discipline as undo.

## Client access to the change log

Two paths, paired:

- **Push (steady state, no new API):** events already carry `actorId` + `changeId`. A client filters
  the event stream to its own `actorId` and maintains its undo-history projection locally as changes
  arrive. REST/NATS already consume events (COV subscriptions), so the live "what can I undo" list is
  free once ids are on events.
- **Pull (cold-start / resync / cross-session):** `getUndoHistory(actorId, limit)` RPC for after a
  reload or a fresh session, or to see history from the same user's earlier session.

```
getUndoHistory @N (actorId :Int32, limit :UInt32 = 50)
  -> (undoable :List(ChangeEntry), redoable :List(ChangeEntry));

struct ChangeEntry {
  changeId     :UInt32;
  opType       :ChangeOp;     # ADD_COMPONENT, REMOVE_EDGE, MOVE, ...
  summary      :Text;         # optional human string; FE may re-render
  affectedUids :List(UInt32);
  createdAt    :UInt64;
  state        :ChangeState;  # APPLIED | UNDONE
  undoable     :Bool;         # false if EXPIRED (purged) or precondition can't hold
}
```

- **Reads the in-memory `ChangeLog` stacks, not SQLite** — no DB round-trip, no writer-thread
  contention. SQLite is durability + boot replay only.
- **Scoped by `actorId`** — you only see/undo your own. `actorId = 0` → all (admin/audit), optional.
- **Not SHM.** Cold, human-rate, durable-in-SQLite data — mirroring it into SHM would be machinery for
  no benefit (hot-path guidance). RPC against the in-memory stacks is the right tool. The browser
  never touches it directly: REST re-exposes the query as an HTTP endpoint, NATS as a
  `get.v0.changelog` subject.

## Extensions as actors (not just FE clients)

Any extension can be an undo actor — the structural write surface already exists as `IEngineWriter`
(`addComponent` / `addEdge` / `removeComponent` / batch ops / `restoreComponent` — the last already
labelled "undo support"). No new write path needed. To enable extension undo:

- Add optional `actorId` to `IEngineWriter` methods, **defaulting to the calling extension's own id**.
  A plain extension (e.g. demo) never passes one — its changes are tracked under its ext id
  automatically; `writer->undo()` reverts its own last structural change.
- Add `undo()` / `redo()` / `getUndoHistory()` to `IEngineWriter` (mirrors the App RPC surface).
- This is why the **ext-id-in-high-bits** `actorId` scheme matters: with extensions as actors,
  multiple write origins coexist in one install (demo + REST, etc.), and the high bits namespace the
  stacks so they never collide. The "REST or NATS, not both" assumption only ever applied to *client*
  origins; extension origins are namespaced regardless.
- **Optional integrity guard:** engine checks the `actorId` high bits match the calling extension, so
  one extension can't undo another's changes. Per-actor scoping already prevents accidental
  cross-undo; this makes it tamper-proof.

## RPC surface (capnp `App`)

- Add optional `actorId :Int32` to writer methods (`addComponent*`, `removeComponent*`, `addEdge*`,
  `removeEdge*`, `moveComponent`, batch ops, override set/clear). `0` = untracked (no undo entry).
- New: `undo @N (actorId :Int32, changeId :UInt32 = 0) -> (result :UndoResult)` and
  `redo @M (actorId :Int32) -> (result :UndoResult)`. `changeId = 0` → latest.
- `UndoResult { ok :Bool, reason :Text, changeId :UInt32, affected :DeletedItems-or-BatchAddResult }`.
- Events gain `actorId` + `changeId` fields.

## Hook points

- `ApplicationImpl` writer handlers — after the mutation succeeds and UIDs are known, build the
  inverse record and enqueue the change-log STORE job. (One helper, called from each writer handler,
  keyed off the assigned UIDs already in hand.)
- `ChangeLog` service (new) — in-memory per-actor stacks + APPLIED/UNDONE/redo bookkeeping, backed by
  the SQLite table. Replay drives inverse ops through the existing `ApplicationImpl` paths under
  `withEvalHold`.
- DB writer — new job type + boot-load of `change_log`.
- Purge callbacks (`engine.cpp` softDeletePurgeLoop + purgeSoftDeleted) — mark entries `EXPIRED`.
- Event emission — carry actor/change ids.

## Client integration

### NATS — source-compat only, no undo surface

NATS (`testExtensions/ce-nats`) **consumes** `IEngineWriter` and reads the event stream; it does not
implement either. The undo changes are additive, so NATS keeps working with **no new subjects and no
undo/redo/changelog exposure**:

- Optional `actorId` defaults to the calling extension's id → NATS writer calls become tracked under
  the NATS ext-id actor automatically, invisibly to NATS clients. No subject signatures change.
- New `IEngineWriter` methods (`undo`/`redo`/`getUndoHistory`) are additive — consumers are unaffected.
- New `actorId`/`changeId` fields on event structs are additive. **Only task:** verify the NATS
  event→JSON serializer tolerates the new fields (ignore or pass through) and that NATS builds and
  behaves against the updated SDK. Patch only if something asserts on struct shape/size.

Explicitly **not** in scope for NATS: `undo`/`redo`/`changelog` subjects, surfacing `actorId`/
`changeId` to NATS clients. Goal is "still works," nothing more.

### REST — full integration, detailed in `NubeExt/ce-rest/openapi.yaml`

REST gets the complete feature, specced in `openapi.yaml`:

- **`actorId` on every writer op.** Recommend an `X-Actor-Id` request header (uniform across all
  writer endpoints, no per-body schema churn): `addNode`, `updateNodeByUid`, `removeNodeByUid`,
  `bulkAddNodes`/`bulkUpdateNodes`/`bulkDeleteNodes`, `copyNodes`, `overrideNodeByUid`, `addEdge`/
  `updateEdge`/`removeEdge`. REST owns per-user `actorId` allocation (ext-id high bits + user).
- **New endpoints** (with full request/response schemas):
  - `POST /undo` — body `{ actorId, changeId? }` → `UndoResult`.
  - `POST /redo` — body `{ actorId }` → `UndoResult`.
  - `GET /changelog?actorId=&limit=` → `{ undoable: [ChangeEntry], redoable: [ChangeEntry] }`
    (the pull path; complements `getRestorableNodes`/`getRestorableEdges` already present).
- **`changeId` echoed** in writer responses and in the COV/event stream REST already pushes, so the
  FE correlates its own change and updates its local undo projection (the push path).
- **New schema components** in `openapi.yaml`: `ChangeEntry`, `UndoResult`, `ChangeOp`, `ChangeState`.

Per the ce-rest working model, this stays self-contained in the extension (its own tests/spec, no
engine-wide test gates).

## Risks

- **Not feasibility — semantics.** Scope creep through input-value undo and conflict edge cases is the
  real risk; v1 stays narrow (structural + override, reject-if-not-clean) to contain it.
- **Replaying inverses must reuse the real writer paths**, not a side door — otherwise undo and normal
  edits diverge in how they wire the graph / persist.
- **Commit ordering and GC-on-purge are the two correctness invariants** to get right; both have a
  safe-degradation story above.
