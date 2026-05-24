## Done

- Added `starter_undo::dispatch::record_if_reversible(registry, recorder, actor, draft)` plus `ChangeDraft` builder; unit tests round-trip a fake `Reversible` through an in-memory recorder and assert unregistered kinds skip silently.
- Added top-level `starter_undo::undo_last(service, actor, scope)` wrapper (scope reserved, currently delegates to `UndoService::undo`).
- Made `chrono` and `serde_json` non-optional deps in `starter-undo/Cargo.toml`.
- Added `rubix_tools::undo::dispatch::{UndoDispatcher, ReversibleTool, ActorSource, StaticActor}` — `Tool`-shaped wrapper that calls inner.invoke then `record_if_reversible` via the tool's `change_for` adapter.
- Implemented `rubix_tools::undo::last::UndoLastTool` for the `rubix.undo.last` verb; pulls actor from an `ActorSource`, returns `{"group_id": ...}`.
- Rewrote `rubix/docs/design/undo/README.md` in present tense covering the three-piece wiring (registry, helper, dispatcher) and the "add a new reversible resource" checklist.
- Added integration test `rubix/crates/rubix-agent/tests/undo_dispatch_test.rs` that wires a fake tool + Reversible through the live `SqliteChangeRecorder` and asserts the recorded row drives the inverse path. Passes.
- Three commits in dependency order on branch `codeless/rubix-goals-2-4-3`: starter-undo → rubix-tools → rubix-agent. Working tree clean. `lint-doc-refs.sh` clean.

## Next

- Stage A.2 (per WORKFLOW) — a fresh session picks it up.

## What you need to know

- `record_if_reversible` returns `Ok(None)` when the resource kind has no `Reversible` registered; this is intentional and matches the read-only/no-op semantics described in `docs/design/undo/`.
- The dispatch helper opens its own `ChangeRecorder::transaction` per call. Multi-row tool effects that need to share a `GroupId` will need a different entry point (not in scope for A.1).
- `UndoLastTool` accepts an unused `scope` JSON object in input so the schema is forward-compatible with the goal-2/3/4 per-resource filter; the field carries `#[allow(dead_code)]`.
- `rubix-tools` Cargo.toml now depends on `starter-undo` and `starter-changelog`; `rubix-agent` dev-deps gained `starter-undo` and `async-trait`.

## Open questions

- Should the agent registry actually wrap existing tools (disk, alert, etc.) with `UndoDispatcher` now, or wait until per-goal stages introduce their first reversible resources? Stage A.1's brief only asks for the seam plus one fake-tool test — left untouched.
- `undo_last`'s `scope` parameter is reserved; later stages need to decide whether the filter lives on `UndoService::undo` (most natural) or stays a thin wrapper in `starter_undo`.
