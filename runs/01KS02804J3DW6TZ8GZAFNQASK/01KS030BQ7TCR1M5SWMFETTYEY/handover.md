## Done

- Implemented synchronous tokio slot propagator in `crates/starter-flow/src/propagator.rs`: `PropagatorConfig { max_propagation_hops }` (default 1000), `FlowTopology { links, triggers, behaviors }`, `spawn()` + `drive()`. R2 chokepoint preserved — propagator never owns slot data, only routes through `GraphStore::write_slot`. R3 short-circuit hooked store-level. Cycle budget enforced per-event with `FlowEvent::RunFailed { CycleBudgetExhausted }` on exceed. Cancel via biased `tokio::select!` so cancel halts scheduling within hundreds of ms; cancel handle also borrowed into every `NodeBehavior::invoke` via `NodeCtx`.
- Added `RunCancel` concrete `Cancel` impl in `crates/starter-flow/src/run.rs` (AtomicBool + `tokio::sync::Notify`, no `tokio_util` dependency).
- Extended `NodeCtx` in `starter-flow-spi/src/node.rs` with `run: RunId`, `node: &NodeId`, `cancel: &dyn Cancel` plus a `new` constructor (replaces the Phase-1 `_phantom` placeholder; struct is still `#[non_exhaustive]`).
- Subscription is created synchronously in `propagator::spawn` before the `JoinHandle` is returned to eliminate a subscribe-vs-first-write race.
- Tests: 11 pass under `cargo test -p starter-flow` — linear three-node chain end-to-end, identity self-loop terminates on R3 short-circuit, Incrementer self-loop terminates on `max_propagation_hops` with `RunFailed`, fired Cancel mid-run stops scheduling within 500ms with `RunCancelled` emitted, plus two `RunCancel` tests. `cargo build --workspace`, `cargo clippy -p starter-flow -p starter-flow-spi --all-targets -- -D warnings`, and `cargo fmt --check` all green.
- Commit `d5124ed` on branch `codeless/starter-flow-engine`.

## Next

- Stage 5 of 12 (a fresh session will pick it up — do not start it here). Based on the job goal this is likely `NodeKindRegistry + FlowRegistry` in `crates/starter-flow/src/registry.rs`, which the propagator currently consumes via the in-test `FlowTopology` shortcut.

## What you need to know

- `FlowTopology::links` is `HashMap<SlotRef, Vec<SlotRef>>` (not `BTreeMap`) because `SlotRef` derives `Hash + Eq` but not `Ord`. `triggers` and `behaviors` are `BTreeMap` keyed by `NodeId` (which is `Ord`).
- `drive()` is also public alongside `spawn()` so Phase 3 engine wiring or smoke tests can run the loop inline with a caller-supplied `SubscriptionStream`.
- The R3 short-circuit lives entirely in `InMemoryGraphStore::write_slot` (stage 3). The propagator deliberately does not duplicate that check — it just stops seeing events.
- Each event consumed counts as one hop. The check is `hops > max_propagation_hops` so with `max=10` the 11th event terminates the run; the test exercises this with an Incrementer-driven self-loop.
- The cancel test uses a `broadcast::channel(4096)` and a `drain_events` helper that skips `RecvError::Lagged` — a tight Incrementer loop overruns the receiver buffer before assertions, and the test only cares about the tail of the stream.

## Open questions

- (none)
