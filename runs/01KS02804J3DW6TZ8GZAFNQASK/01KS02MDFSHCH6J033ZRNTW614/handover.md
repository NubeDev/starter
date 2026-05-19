## Done

- Implemented `InMemoryGraphStore` in `crates/starter-flow/src/graph.rs` per stage 3: `tokio::sync::RwLock<BTreeMap<NodeId, Node>>` storage + per-store `broadcast::Sender<GraphEvent>`; `GraphStore::write_slot` is the single chokepoint with R2 replay semantics (replay=true suppresses `SlotChanged` publish), R3 idempotent-write short-circuit, and `WriteSlotOpts.force` to defeat it; every write opens a `write_slot` tracing span recording `node_id`, `slot_name`, `replay_flag`, `force_flag`, `prev_was_equal`.
- `subscribe` returns `BoxStream<GraphEventEnvelope>` backed by `tokio_stream::wrappers::BroadcastStream`; lagged batches are silently dropped (operator-facing surfacing deferred to Phase 7).
- SPI: added `WriteSlotOpts.force: bool` + `WriteSlotOpts::forced()`, and `GraphEventEnvelope::slot_changed(slot, value)` constructor (both fit through `#[non_exhaustive]` non-breakingly).
- Wired `tokio-stream` into the workspace and into `starter-flow` (with `tracing`, `async-trait`, `futures`); dev-deps gain `tokio` `macros` + `time` for the unit tests.
- 5 unit tests green: single-writer → one event, replay → zero events, idempotent short-circuit, force defeats short-circuit, subscribe-after-only.
- Closing trio green: `cargo test -p starter-flow`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`.
- Committed as `stage 3 — in-memory GraphStore impl in crates/starter-flow/src/graph.rs` (c926709).

## Next

- Stage 4: the synchronous tokio slot propagator (R2 + the rubix live_wire model) — subscribes to the per-store broadcast and walks outbound `Link`s, honouring `max_propagation_hops` (D1a default 1000) and `idempotent_short_circuit` (already enforced inside `write_slot`).

## What you need to know

- `GraphEvent` is engine-internal (only `SlotChanged` today, `#[non_exhaustive]`); the SPI wire type is `GraphEventEnvelope`. The propagator can subscribe via the SPI `subscribe()` and consume envelopes; it does not need to depend on `GraphEvent` directly.
- The R12 safe-state drive path lives **outside** `write_slot` and **must publish** `SlotChanged` — the in-memory store does not have a special path for this; the engine module will call `write_slot` with `WriteSlotOpts::live()` (or `forced()` if it needs to defeat R3 for a transition to the same value).
- `read_slot` returns `GraphError::UnknownSlot` for both unknown node and unknown slot — collapsed for Phase 2; split if a kind needs the distinction.
- Three pre-existing Cargo warnings about `default-features = false` on `workspace.dependencies` entries (starter-flow, starter-flow-nodes, starter-flow-surfaces) are unchanged from prior stages and out of scope.

## Open questions

- (none)
