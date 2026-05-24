# UNDO

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

## Tests

- **`starter_undo::dispatch::tests`** — unit-level round-trip
  through a fake `Reversible` and an in-memory recorder.
- **`rubix_agent` integration test `undo_dispatch_test.rs`** —
  registers a fake tool + Reversible, dispatches through the live
  `SqliteChangeRecorder`, and asserts the recorded row drives the
  inverse path.
