# `starter-flow` — Hot-reload of definitions

Companion to [SCOPE.md](SCOPE.md). Resolves **D3 — Hot-reload of
flows**: a CRUD edit to a flow's node settings, links (edges), or
the set of nodes themselves must take effect without restarting the
process, without inventing a new write path, and without dropping
in-flight runs on the floor.

## One-line summary

**Every definition edit is a new immutable `FlowRevision` written
through one chokepoint; the engine atomically swaps the resolved
`Arc<FlowTopology>` for that flow; in-flight runs finish on the
snapshot they started with; new runs pick the new revision; settings
edits short-circuit to a slot write and never rebuild the topology.**

This is the same posture R2 takes for *data* (one `write_slot`
chokepoint, slot-equality short-circuit) lifted up to flow
*definitions*: one `FlowStore::put` chokepoint, revision-equality
short-circuit.

## Why this exists

Today (Phase 2–5):

- `FlowTopology` is constructed once at boot and frozen
  (`Arc<FlowTopology>` cloned per run — see
  [`examples/notes/src/flow_demo.rs`](../../../examples/notes/src/flow_demo.rs#L60),
  comment: *"Frozen topology — same three nodes, same links, every
  run"*).
- [`FlowRegistry::put`](../../../crates/starter-flow/src/registry.rs#L241)
  records an immutable revision but nothing resolves it to a
  `FlowTopology`, and nothing tells the runtime *"use the new one
  now"*. The Phase 3 SCOPE-named `FlowRegistry::resolve` does not
  yet exist.
- [`NodeKindRegistry::register` / `deregister`](../../../crates/starter-flow/src/registry.rs#L113)
  exists, but a deregister never invalidates topologies that
  reference the kind.
- "Save" in any editor surface is a no-op for the running engine.
  The operator restarts the process to see their change.

The cost: every settings tweak ("change a prompt", "raise a
`cost_cap`"), every wiring change ("add an `http-out` after the
agent"), and every node-set change ("drop the `log` node") is a
deploy-cycle, not an interactive edit. That kills the authoring
shape the SCOPE Why-this-exists block calls out — *"Node-RED's
model is the right authoring shape for AI + integration workflows
in 2026"*. Node-RED's edits are interactive; if starter's are not,
the SCOPE's third force collapses.

## Hard rules (load-bearing)

These rules govern the hot-reload subsystem. Each is the
analogue of an existing SCOPE rule lifted up one level — from
*data writes* (slots) to *definition writes* (revisions).

### HR1 — One definition-write chokepoint

Every definition edit — REST handler, CLI command, UI canvas
save, host-dir file-watch, extension reload, programmatic API —
funnels through one function:
`DefinitionManager::publish(flow_id, draft) -> FlowRevisionId`.

`publish` is the only writer that:

1. Validates the draft (R10 namespace ownership, kind ids
   resolve in `NodeKindRegistry`, link types compatible, no
   `release` safe-state on kinds that don't support it per R12).
2. Canonicalises the body per **RFC 8785 (JCS)** — UTF-8,
   sorted object keys, no insignificant whitespace, fixed
   number formatting — then computes a `blake3` hash over the
   canonical bytes. Canonicalisation is non-negotiable: two
   editors that emit semantically-equal JSON in different key
   orders must produce the same hash, otherwise the
   idempotent short-circuit silently breaks and `FlowStore`
   accretes duplicate revisions.
3. Looks up the flow's current head; if the hash matches the
   head's hash, returns the head id unchanged (idempotent
   short-circuit — the definition-level analogue of R2's
   slot-equality short-circuit).
4. Allocates a fresh `FlowRevisionId`, writes
   `FlowStore::put(revision)`, and emits one
   `FlowDefinitionEvent::RevisionPublished { flow, revision,
   prev_head, kind }` on the engine's definition bus.

**Reason: with one publish path, every invariant the engine
cares about (validation, auth via `Principal`, audit, replay,
two-phase commit per [extensions R3](../../extensions/scope/SCOPE.md))
is enforced in one place.** A REST endpoint that bypasses
`publish` to write `FlowStore` directly is a stage-fail in
the same shape R2's grep-contract test enforces for
`write_slot`.

### HR2 — Revisions are immutable; the active pointer is mutable

A `FlowRevision` is never edited in place. An edit produces a
new revision (HR1). The engine keeps one *mutable* per-flow
pointer — `ActiveTopology = ArcSwap<FlowTopology>` — that names
which revision is *active* right now.

- In-flight runs hold an `Arc<FlowTopology>` snapshot they
  obtained when their `FlowRunner::start` was called. They
  finish on that snapshot. (Same posture as today's `RunSpec.topology`.)
- New runs that begin after `ActiveTopology::store(new)` see
  the new topology.
- The pointer swap is `ArcSwap::store`, which is wait-free for
  readers (the propagator's hot path) and bounded for the
  writer.

This is what makes hot-reload **safe by construction** — no run
ever observes a half-built topology, no two runs ever disagree
about which links exist mid-tick, and the immutable-revision
invariant from the SCOPE "Decisions made" block holds verbatim.

### HR3 — Three edit kinds; three application paths

CRUD edits classify into three kinds. The classifier is pure
(diff `old_body` against `new_body`); the engine picks the path
automatically.

| Kind | What changed | Application path | Touches `FlowTopology`? |
|---|---|---|---|
| **Settings-only** | One or more `config_slot` values; nothing else | `GraphStore::write_slot(config_slot, new_value, WriteSlotOpts::config())` per changed slot | **No** |
| **Structural** | Links added / removed / re-targeted; nodes added / removed; kind ids changed | Build a new `FlowTopology`; `ActiveTopology::store(Arc::new(t))` | **Yes** |
| **Mixed** | Both | Structural path **then** the settings path for every config-slot delta (in that order) | **Yes** |

**Settings-only** is the common case (operator tweaks a prompt,
raises a cost cap, flips a `session_policy`). It hits the same
`write_slot` chokepoint R2 already governs, with a new
`WriteSlotOpts::config()` flag (analogue of the existing
`WriteSlotOpts::replay`) that:

- Marks the write as a definition-origin write in the per-write
  tracing span (`origin = "definition"`).
- Does **not** suppress the `SlotChanged` event — downstream
  nodes that subscribe to a config slot's value (the common
  reactive-Rubix shape) react immediately.
- Is honoured by the propagator's idempotent-write
  short-circuit (R2): writing the same value is a no-op.

**Structural** edits must rebuild the topology because
`FlowTopology.links` / `triggers` / `behaviors` are immutable
maps inside an `Arc`. Build-and-swap is cheap (HashMap +
BTreeMap construction, on the order of ms for graphs Codeless
and Rubix care about) and avoids the alternative of nesting an
`ArcSwap` per field — which would let a tick read a mismatched
`(links, behaviors)` pair.

**Mixed** is **not** a simple roll-up to structural. The
topology swap alone *embeds* the new config-slot defaults in the
new topology but does not fire `SlotChanged` for the values
already resident in `GraphStore` — reactive Rubix-shape
subscribers wired to those slots would silently miss the edit
until something else wrote them. The Mixed path therefore runs
the structural swap first (so new runs see the new wiring) and
then runs the settings-path `write_slot` per changed config slot
(so the existing slot store fires `SlotChanged` and reactive
subscribers re-tick). Order matters: structural-then-settings
guarantees the writes land into the new topology's slot graph,
not the old one's. The idempotent short-circuit on `write_slot`
(R2) drops any setting whose value already matches the live
store.

### HR4 — `apply_policy` is read from the flow definition, not owned by the engine

Per R3 ("the engine is a reader of policies, never an owner"),
the policy for *how* a structural change applies to in-flight
runs lives as a flow-level config slot, not as engine code:

```yaml
flow_id: examples.notes.codeless-demo
apply_policy: drain     # drain | restart | live-migrate
```

| Value | Behaviour on structural edit |
|---|---|
| `drain` (default) | In-flight runs finish on their snapshot. New runs use the new topology. Codeless-shape default — short-lived staged jobs don't benefit from mid-run topology changes. |
| `restart` | Engine fires each in-flight run's `RunCancel` (R13). R12 safe-state walks each affected writable output. New runs (re-triggered by the caller or by `trigger.event`) use the new topology. Rubix-shape default for long-lived reactive flows where the new topology *is* the operator's intent. |
| `live-migrate` | Engine compares old and new topology. Settings deltas write through `write_slot` into in-flight runs **only when the affected config slot is owned by a node whose `(NodeId, KindId, inbound link set, outbound link set)` is byte-identical between old and new topologies** — i.e. the slot's semantic ownership did not change. Every other delta (structural, *or* a settings delta on a node whose wiring shifted) falls back to `restart`. For graphs where most edits are settings, this is the lowest-disruption option; but the safety bar is "the in-flight snapshot's behaviors must still be operating on the same logical slot they thought they were", which means the migration is best-effort by classification, not by aspiration. The classifier is pure and conservative — when in doubt, falls back to `restart`. |

`apply_policy` is read from the **previous** revision (the one
the in-flight runs are using) — the new revision can't dictate
how the old one is torn down. Engine has no `match` arm over
the policy names per the R3 grep-contract.

**First-publish corner case.** When there is no previous
revision (the very first `publish` for a `FlowId`), there are
by construction no in-flight runs against the flow, so no
policy applies — the swap is unconditionally an atomic mount.
**Policy-edit corner case.** A publish whose delta includes
`apply_policy` itself is governed by the *old* value (the
in-flight runs are on the old revision, which carries the old
policy). The new value applies to the *next* edit. An operator
flipping `drain → restart` to interrupt a stuck in-flight run
must either publish the policy change first and then publish
the real edit, or cancel the in-flight run explicitly via the
run-control surface. Documented because the surprise is
asymmetric: "I set restart and the engine still drained" is
the report you'll get otherwise.

### HR5 — Boot is "resume to last known good"

On engine boot, for every `FlowId` known to the host:

1. `FlowStore::head(flow)` → `head_rev`. If `None`, the flow
   has never been published — skip (it'll appear when
   `publish` runs).
2. `FlowStore::load(flow, Some(head_rev))` → body.
3. `TopologyResolver::resolve(&body, &NodeKindRegistry)` →
   `Arc<FlowTopology>` (or a validation error — see HR6).
4. `engine.active_topologies.insert(flow, ArcSwap::new(t))`.
5. If the host-dir watcher is configured (HR7), walk the dir
   once at boot and re-publish any file whose on-disk
   `blake3` differs from the `FlowStore` head's hash. HR1's
   idempotent short-circuit makes this a no-op when they
   match.

Then `RunStore::list_open()` per R6 drives the resume-from-
checkpoint walk — runs that were in-flight at the previous
SIGTERM resume on whatever revision their checkpoint records
(stored in the checkpoint's `flow_revision` field, already
present per [Phase 3 R6](SCOPE.md)). A resumed run is
**explicitly not** auto-migrated to the new head; the operator
who edited the flow may not have intended for an in-flight run
from yesterday to absorb the change. If they did, they cancel
the resumed run and let `trigger.*` re-fire on the new head.

**Revision liveness across boot publishes.** Step 5's
file-watch re-publish may advance `head_seq` for a flow whose
resumed runs reference an older revision. This is safe by
construction: `FlowStore` revisions are append-only (SCOPE
"Decisions made": *"revisions are immutable; `head_seq`
pointer per flow tracks the current revision"*), so
`FlowStore::load(flow, Some(old_rev))` continues to return the
old body after any number of new publishes. No revision GC
runs while open runs exist; the engine never deletes a
`FlowRevision` row that is still named by an unfinished
`runs.flow_revision` foreign key, period. Boot ordering is
free to publish-from-disk before or after the resume walk
for exactly this reason.

### HR6 — Bad drafts never go live; `last_good` is the fallback

A draft that fails validation (HR1 step 1) or whose
`TopologyResolver::resolve` errors (e.g. the kind id was
deregistered between draft and publish) **does not become a
revision**. `publish` returns the validation error to the
caller; `FlowStore` is untouched; `ActiveTopology` is
untouched. The engine continues serving the previous head.

The same two-phase pattern [extensions R3](../../extensions/scope/SCOPE.md)
uses for kind registration — *"one bad kind never poisons the
registry"* — lifts verbatim: one bad revision never poisons the
flow.

If `FlowStore` itself is degraded (per `EngineHealth::Degraded`
from D-F3.11), `publish` returns `EngineError::BackendUnavailable`
and the in-memory `ActiveTopology` stays at whatever revision was
last successfully published. Boot in this state serves whatever
`FlowStore::load` does return; flows that fail to load surface
as `FlowDefinitionEvent::ResolveFailed { flow, error }` and stay
unmounted until either the backend recovers or an operator
re-publishes from a file source.

### HR7 — File-watch is one publisher among many, not a special case

A host-dir watcher (default
`$XDG_DATA_HOME/<binary>/flows/`, per
[SCOPE D2](SCOPE.md#open-questions)) is **just another caller of
`publish`** — not a parallel write path:

1. `notify`-backed watcher fires on file change (debounced
   200 ms by default to coalesce editor-saves; tunable via
   `starter-flow-watch` config because some editors
   atomically rename-then-truncate over windows up to ~1 s
   on networked filesystems).
2. Parser reads the YAML, normalises to JSON canonical form.
3. Calls `DefinitionManager::publish(flow_id, body, source:
   FileSource { path })`.
4. HR1's idempotent short-circuit drops no-op edits silently
   (an editor that "touches" a file without changing bytes
   doesn't churn the registry).

Same chokepoint, same validation, same audit trail. Removing
a file deletes the flow via `publish_delete(flow_id)` — a
fourth method on `DefinitionManager` that emits
`FlowDefinitionEvent::Removed` and removes the
`ActiveTopology` entry. Per HR4, in-flight runs drain.

The watcher is optional; CLI / REST / extensions can publish
without it. Hosts that don't want filesystem coupling don't
enable it; the engine surface is identical either way.

### HR8 — Node-kind add/remove is symmetric

When [`NodeKindRegistry::register`](../../../crates/starter-flow/src/registry.rs#L113)
adds a kind, the engine re-attempts `TopologyResolver::resolve`
for every flow currently in
`FlowDefinitionEvent::ResolveFailed` state (HR6 fallback) whose
failure mode was *unknown-kind* for the newly-registered id.
Successful resolves transition those flows into a live
`ActiveTopology` and emit `FlowDefinitionEvent::Mounted`.

When [`NodeKindRegistry::deregister`](../../../crates/starter-flow/src/registry.rs#L165)
removes a kind, the engine:

1. Walks every `ActiveTopology` for nodes whose `KindId`
   matches.
2. For each match, applies the flow's `apply_policy` (HR4):
   `drain` lets in-flight runs finish on the snapshot they
   already hold (which still has a valid `Arc<dyn
   NodeBehavior>` because behaviors live inside the
   topology snapshot, not in the registry); `restart`
   cancels and tears down; `live-migrate` falls back to
   `restart` because deregister is structural.
3. Transitions the flow's `ActiveTopology` into a
   `ResolveFailed` state; the flow re-mounts only when the
   kind is re-registered (HR8 first paragraph).

**Memory-safety constraint on out-of-process / dylib-backed
kinds.** A snapshot held by an in-flight run carries an
`Arc<dyn NodeBehavior>` whose vtable points into the code
that shipped the kind. For in-process built-in kinds and
statically-linked extension kinds this is fine — the code
stays mapped for the process lifetime. For dylib-loaded
extension kinds (Phase 6 `starter-ext-flow` wasm or native
dylib flavours) and for process-flavour extension kinds
(where the `NodeBehavior` impl is a JSON-RPC stub whose
drop closes the child's stdio handle), `deregister` must
**not** cause the underlying code or transport to disappear
while any snapshot still references it. The contract:

- `deregister` removes the kind from the registry's lookup
  map (no new resolves can pick it up) but does **not** by
  itself drop the `Arc<dyn NodeBehavior>` the registry was
  holding.
- The supervisor / dylib loader **must** wait until
  `Arc::strong_count` on the behavior reaches one (the
  registry's own retained reference) before unmapping the
  code or tearing down the child process. The wait is
  bounded by the flow's `apply_policy`: `restart` cancels
  in-flight runs immediately so the wait is short; `drain`
  waits for the longest in-flight run to finish;
  `live-migrate` falls back to `restart` for deregister
  (above), so it behaves as `restart`.
- Hosts that need bounded unload latency configure
  affected flows with `apply_policy: restart` for the
  kinds they intend to ship from unloadable extensions.
  This is a deployment-time policy choice, not an engine
  invariant.

The consequence: `drain` is the safe-by-default policy for
in-process kinds and the slow-by-default policy for
dylib-backed kinds. The SCOPE R11 "capability discipline"
promise (extension reload is safe) holds only when the
supervisor honours this strong-count wait; if it doesn't,
you get a vtable-into-freed-code segfault six months into
production, not at deregister time. Phase HR-6 ships the
wait shape; the supervisor / dylib loader integrations
implement it.

## What this means for CRUD shapes

Mapping the user's question — *"reactive/hot reload or how it
is now crud and save"* — onto the rules above:

- **CRUD of node settings** → HR3 settings path. UI saves a
  config-slot change → `publish` is called with a draft whose
  only delta is config slots → diff classifier picks the
  settings path → engine performs `write_slot` per changed
  slot → downstream nodes react via R2 immediately. **No
  topology swap; no run interruption; instant.**
- **CRUD of edges** → HR3 structural path. UI adds a link from
  `agent.output` to `log.value` → `publish` validates type
  compatibility → new revision written → `TopologyResolver::resolve`
  builds a topology whose `links` map includes the new entry →
  `ActiveTopology::store` swaps it in → new runs use the new
  wire; in-flight runs honour the flow's `apply_policy`.
- **CRUD of nodes** → HR3 structural path, same as edges. Adding
  a node grows `behaviors` + `triggers`; removing one shrinks
  them. Same swap, same `apply_policy`.
- **"Reactive vs save"** → HR1 collapses the distinction. Save
  *is* the reactive event: `publish` is synchronous, the swap
  is wait-free, and an operator who clicks "save" sees the
  next-fired run pick up the change with no extra step. A UI
  that wants a "draft / publish" two-step (e.g. to allow
  multi-field edits before committing) implements it on top
  of `publish` — the engine doesn't need to know.

## Relationship to existing crates

```
starter-flow-spi  (existing)
   FlowStore, FlowRevision, FlowEvent, RunStore, RunOpts
        ▲
        │  no changes to existing trait shapes
        │  add: FlowDefinitionEvent, DefinitionSource,
        │       TopologyResolverError, ApplyPolicy enum
        │
starter-flow      (existing engine)
   add: DefinitionManager (HR1 chokepoint)
   add: TopologyResolver (FlowRevision.body → FlowTopology)
   add: ActiveTopology = ArcSwap<FlowTopology> per FlowId
   add: definition bus (broadcast::Sender<FlowDefinitionEvent>)
   modify: FlowRunner::start reads from active_topologies[flow]
           instead of taking a hand-built Arc<FlowTopology>
   modify: NodeKindRegistry::deregister fires HR8 walk
        ▲
        │
starter-flow-watch  (NEW — optional, default-off cargo feature)
   notify-backed file watcher; one caller of
   DefinitionManager::publish per file event. No new wire
   format, no new persistence.
        ▲
        │
starter-store-sqlite  (existing, behind "flow" feature)
   FlowStore impl already exists; no schema change required
   (revisions table already keyed by FlowRevisionId).
```

The `ArcSwap` dep is new for `starter-flow` (`arc-swap` crate;
zero unsafe in our use). The `notify` dep is new for
`starter-flow-watch` and is paid only by hosts that enable
file-watch.

## Observability

Every transition is a `tracing` span, same shape as R12's
`engine.transition`:

- `flow.definition.publish` — fields: `flow`, `revision`,
  `prev_head`, `source`, `kind` (`settings | structural | mixed`),
  `outcome` (`published | short_circuited | rejected`).
- `flow.definition.swap` — fields: `flow`, `from_revision`,
  `to_revision`, `apply_policy`.
- `flow.definition.resolve_failed` — fields: `flow`, `error`,
  `source`.
- `flow.definition.kind_revoked` — fields: `flow`, `kind`,
  `apply_policy`, `cancelled_runs`.

Metrics: `flow_definition_publishes_total{outcome}`,
`flow_definition_swaps_total`,
`flow_definition_active_topologies`,
`flow_definition_resolve_failures_total`.

Replay/audit: the `FlowStore` revisions table already records
every revision (it's append-only). Adding a `source` column
(`api | cli | file:<path> | extension:<id>`) makes the audit
trail a single SQL query.

## What does NOT land

- **No partial topology mutation.** No "add this one link to
  the live `FlowTopology`" API. HR2 is load-bearing: the
  swap is atomic-or-nothing.
- **No cross-flow transactional publishes.** Each
  `publish` writes one flow. A consumer that needs
  "publish flow A and flow B atomically" composes it at the
  caller layer; the engine offers single-flow atomicity only.
- **No automatic migration of in-flight runs to new
  topology.** `apply_policy: live-migrate` does this for the
  settings-only delta; everything else is drain or restart.
  An in-flight run grafted onto a structurally different
  topology mid-tick is a class of bug nobody wants to debug.
- **No revision rollback UI.** "Rollback" is "publish the
  body of an older revision again" → HR1 produces a new
  revision id whose body matches the old one. The history
  is preserved; the active pointer moves forward, never
  backward, which keeps the audit story simple.
- **No engine-level rate limit on publishes.** A misbehaving
  caller that fires `publish` in a hot loop pays the
  `blake3` + validation cost per call; idempotent
  short-circuit prevents `FlowStore` churn. If a real
  workload surfaces a need, the rate limit lands as a
  config slot on the flow definition itself per R3, not as
  engine code.
- **No hot-reload of the `NodeKindRegistry` for built-in
  kinds.** Built-ins (`starter.flow.*` per R10) ship with
  the binary and only change on process restart. Extension-
  contributed kinds reload via the extensions adapter
  (which already exists per R11) and trigger HR8.

## Decisions made

- **D-HR1 — `ArcSwap<FlowTopology>` over `RwLock<Arc<FlowTopology>>`.**
  Reader hot path is the propagator's per-tick topology read; wait-
  free `ArcSwap::load` beats `RwLock::read` on the contention shape
  reactive Rubix-style flows have (every slot change reads the
  topology once). Writer path is `publish`, which is not on the
  hot path. The `arc-swap` crate is a single small dep with no
  unsafe in our use.
- **D-HR2 — Settings-only edits go through `write_slot`, not
  through a topology swap.** R2 already governs writes to config
  slots (the propagator subscribes to them, downstream reacts).
  Forcing settings edits through a topology rebuild would (a) be
  wasteful, (b) break the reactive shape Rubix-style flows
  depend on (a config slot bound to a downstream node would
  trigger that node's re-invocation via topology swap rather
  than via `SlotChanged`). One write path, no second mechanism.
- **D-HR3 — `apply_policy` defaults to `drain`.** Matches the
  Codeless shape (short-lived staged runs that don't benefit
  from mid-run topology changes), which is the more common shape
  in the workspace today. Rubix-shape flows opt into
  `live-migrate` or `restart` explicitly via a config slot on the
  flow — same R3 pattern as `session_policy` / `on_failure`.
- **D-HR4 — The diff classifier is pure and lives in
  `starter-flow`.** Input: `(old_body, new_body)`. Output: enum
  `EditKind { SettingsOnly { writes: Vec<(SlotRef, SlotValue)> }
  | Structural | Mixed }`. Pure function, easy to test
  exhaustively. Lives next to `TopologyResolver` because the two
  share the JSON-shape knowledge.
- **D-HR5 — File-watch ships as a separate crate
  (`starter-flow-watch`).** Keeps `starter-flow` free of the
  `notify` dep for hosts that don't want filesystem coupling
  (the headless-appliance posture, SCOPE 735). Mirrors the
  pattern Phase 4 used with `ai-agent` behind a feature, and
  Phase 5 used with `trigger.explicit`.
- **D-HR6 — Boot is the same code path as a live publish.** The
  boot walk calls `DefinitionManager::publish` for each
  file-source flow with `idempotent_short_circuit = true`. No
  parallel "load from disk at boot" path. Same chokepoint, same
  validation, same audit row.

## Open questions

- **Q-HR1 — Draft persistence.** Do drafts (unpublished edits)
  live anywhere durable, or only in the editor's local state?
  Probable answer: local-state only. A consumer who needs
  server-side drafts adds a `FlowDraftStore` trait alongside
  `FlowStore`; the engine doesn't need it for hot-reload itself.
- **Q-HR2 — Per-user / per-tenant active pointers.** Today the
  active pointer is global per `FlowId`. A multi-tenant deployment
  may want "tenant A is on revision 5, tenant B is on revision 6
  while QA-ing it". Defer until a multi-tenant consumer surfaces
  the need — at which point `ActiveTopology` becomes keyed by
  `(FlowId, TenantKey)` additively. **Forward-compat note:** the
  public engine API for resolving a flow to its active topology
  must therefore not bake `FlowId`-as-sole-key into a return
  type or trait method shape that can't grow a tenant parameter
  later. The Phase HR-1 surface (`engine.active_topologies[flow_id].load()`)
  is fine as an internal call but should not appear in the
  public `Engine` API; consumers go through a
  `resolve_active(flow_id, &RunSpec)` accessor that can
  additively start consulting the spec for tenant info.
- **Q-HR3 — Editor-driven `live-migrate` UX for structural
  edits.** A canvas that lets the user "preview" a structural
  edit before publishing could spin up a one-shot run against an
  in-memory `FlowTopology` without touching `FlowStore`. Belongs
  in `starter-ui-flow` (SCOPE D5), not here.
- **Q-HR4 — `RunStore`-aware publish.** Should `publish` know
  about open runs (e.g. to refuse a destructive structural edit
  while runs are mid-flight)? Probable answer: no — `apply_policy`
  is exactly the right place for that policy, and the engine
  shouldn't grow a second knob with overlapping semantics.

## Smoke tests (before merging)

### "Settings edit is one slot write"

A flow has an `ai-agent` node with `cost_cap: 0.10`. An operator
publishes a draft whose only delta is `cost_cap: 0.25`. Tracing
records:
- one `flow.definition.publish` span with `kind = "settings"`,
- one `graph.write_slot` span with `origin = "definition"` on
  the `cost_cap` config slot,
- **no** `flow.definition.swap` span.

If a topology swap fires, HR3 has slipped.

### "Structural edit drains in-flight runs"

A flow with `apply_policy: drain` has run R1 in flight (paused
mid-`ai-agent`). An operator publishes a draft that adds an
`http-out` node downstream of the agent. R1 continues and
completes against the old topology (no `http-out` node fires).
A subsequent run R2 sees the new topology and the `http-out`
node fires.

If R1 starts firing the new node mid-run, HR2 / HR4 has slipped.

### "Bad revision never goes live"

An operator publishes a draft referencing a `KindId` that is
not registered. `publish` returns `TopologyResolverError::UnknownKind`.
`FlowStore::head` is unchanged. `ActiveTopology` is unchanged.
The next run uses the previous head.

If `FlowStore` accepted the bad revision, HR6 has slipped.

### "Idempotent publish is a no-op"

An operator publishes the same body twice. The second call
returns the same `FlowRevisionId` as the first; no new row in
`FlowStore`; no `flow.definition.swap` span; no
`FlowDefinitionEvent::RevisionPublished`.

If a duplicate body produces a new revision, HR1's
short-circuit has slipped.

### "File-watch is just another publisher"

A flow file on disk is edited externally. The watcher fires;
`DefinitionManager::publish` is invoked with `source =
FileSource { path }`. The audit row records the source. The
swap behaves identically to a REST-initiated publish (same
`flow.definition.publish` span shape, same `swap` span shape).

If file-watch takes a different code path, HR7 has slipped.

### "Kind deregister revokes affected flows"

A flow uses kind `com.acme.weather.current`. The extension
contributing the kind is unloaded.
`NodeKindRegistry::deregister` fires. The flow's
`ActiveTopology` transitions to `ResolveFailed`. In-flight
runs honour the flow's `apply_policy`. A new attempt to fire
the flow returns `EngineError::FlowNotMounted`. Re-loading the
extension re-registers the kind; the flow's `ActiveTopology`
re-mounts; firing succeeds.

If a deregister leaves a stale `Arc<dyn NodeBehavior>` in a
live topology that subsequent runs consume, HR8 has slipped.

### "Mixed edit fires SlotChanged for the config delta"

A flow has an `ai-agent` node with `prompt: "old"` and one
outbound link `agent.output → log.value`. An operator
publishes a draft that (a) changes the prompt to `"new"`
and (b) adds a second outbound link `agent.output →
http_out.body`. Tracing records, in order:

- one `flow.definition.publish` span with `kind = "mixed"`,
- one `flow.definition.swap` span (structural part),
- one `graph.write_slot` span with `origin = "definition"`
  on the `prompt` config slot, firing `SlotChanged` for any
  reactive subscriber.

If the `write_slot` span is missing — i.e. the structural
swap was treated as sufficient for the config delta —
HR3's Mixed semantics have slipped and reactive Rubix-shape
subscribers will silently miss the prompt change.

### "Live-migrate falls back to restart when wiring shifts"

A flow has `apply_policy: live-migrate`. Run R1 is in flight.

- **Sub-case A**: operator changes a config slot on a node
  whose `(NodeId, KindId, inbound link set, outbound link
  set)` is unchanged. R1 observes the slot write via
  `SlotChanged` and continues. No `RunCancel` fires.
- **Sub-case B**: operator changes a config slot on a node
  whose outbound link set also changed in the same publish.
  R1's `RunCancel` fires; R12 safe-state walks R1's writable
  outputs; R1 reports `Cancelled`. A subsequent new run uses
  the new topology.

If sub-case B silently mutates the in-flight snapshot's slot
store without cancelling, HR4 live-migrate has slipped.

### "Extension unload waits for snapshots to drop"

A flow uses a process-flavour extension-contributed kind.
Run R1 is in flight (holds a snapshot containing an
`Arc<dyn NodeBehavior>` that proxies to the extension
child over JSON-RPC). The extension is requested to unload.
The supervisor calls `NodeKindRegistry::deregister`. The
registry's lookup is updated immediately; new resolves for
the kind fail. The supervisor's child-termination path
**blocks** on `Arc::strong_count(&behavior) == 1` (the
registry's retained reference is the only one left).
R1 finishes; its snapshot drops; the strong count drops to
one; the supervisor releases its retained reference and
terminates the child. No JSON-RPC call lands on a closed
stdio pair; no vtable-into-freed-code dispatch occurs.

If the supervisor terminates the child before the strong
count reaches one, HR8's memory-safety constraint has
slipped.

### "Boot resumes to last-known-good"

The process is `SIGTERM`'d while a run is mid-flight. The
process restarts. `FlowStore::head` returns the same revision
that was active. `RunStore::list_open` returns the in-flight
run. The run resumes against the revision its checkpoint
records (which may or may not be the new head). A subsequent
new run uses the new head.

If boot mounts a different revision than `FlowStore::head`,
HR5 has slipped.

## Phasing

Each phase independently mergeable; each ships its own
dep-tree gate per the SCOPE Phase 2 D1h precedent.

### Phase HR-1 — `TopologyResolver` + `DefinitionManager`

- `TopologyResolver::resolve(&FlowRevision, &NodeKindRegistry)
  -> Result<Arc<FlowTopology>, TopologyResolverError>`.
- `DefinitionManager::publish(flow_id, body, source) ->
  Result<FlowRevisionId, _>` (HR1 chokepoint).
- `ActiveTopology = ArcSwap<FlowTopology>` per `FlowId`,
  held on the `Engine`.
- `FlowRunner::start` reads from
  `engine.active_topologies[flow_id].load()` instead of taking
  a hand-built topology.
- Smoke: "idempotent publish is a no-op" + "bad revision never
  goes live".

### Phase HR-2 — `EditKind` classifier + settings path

- Pure diff: `classify(old, new) -> EditKind`.
- Settings path: `apply_settings_only(writes)` calls
  `write_slot` per delta with `WriteSlotOpts::config()`.
- Structural path: `apply_structural(new_topology, apply_policy)`.
- Smoke: "settings edit is one slot write" + "structural edit
  drains in-flight runs".

### Phase HR-3 — Definition bus + observability

- `FlowDefinitionEvent` enum + `broadcast::Sender` on
  `Engine`.
- `flow.definition.*` tracing spans.
- Metrics surface.
- Audit `source` column on `FlowStore` revisions table.

### Phase HR-4 — Boot resume + `apply_policy`

- `Engine::start` walks `FlowStore::list` → resolves each
  head → mounts.
- `apply_policy` config-slot read on every structural swap.
- Smoke: "boot resumes to last-known-good".

### Phase HR-5 — `starter-flow-watch` adapter

- New crate, `notify`-backed, default-off cargo feature.
- One caller of `DefinitionManager::publish` per debounced
  file event.
- Smoke: "file-watch is just another publisher".

### Phase HR-6 — `NodeKindRegistry` revoke walk

- `deregister` triggers HR8 walk.
- `register` re-attempts `ResolveFailed` flows.
- Smoke: "kind deregister revokes affected flows".

## Bottom line

**Hot-reload is not a new subsystem; it is the SCOPE's
existing chokepoints (R2 on slots, immutable revisions on
definitions, R3 policy-as-config-slot) wired together with
one new chokepoint (`DefinitionManager::publish`) and one
new piece of mutable state (`ActiveTopology = ArcSwap`).**
Settings edits ride the existing `write_slot` path —
reactive by default, instant, no topology rebuild. Structural
edits build a new immutable topology and atomically swap it,
with in-flight runs honouring a per-flow `apply_policy`
(drain / restart / live-migrate) the engine reads from the
flow definition rather than owning. File-watch, REST, CLI,
UI canvas, and extension reloads are all callers of one
publish function — same validation, same audit, same swap.
Boot is just a `publish` per known flow with idempotent
short-circuit. Bad drafts never go live; in-flight runs
never observe a half-built topology; the operator who
clicks "save" sees the next run pick up the change with no
extra step.

This resolves [SCOPE D3](SCOPE.md#open-questions) and turns
the workspace's flow story from "edit and restart" into
"edit and observe" — the authoring shape the SCOPE's third
force calls for.
