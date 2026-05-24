## Done

- Added `starter-flow-spi::state` module: `NodeStateStore` trait, `NodeStateKey` (256-byte cap), `NodeStateValue` (64 KiB cap), `NodeStateError` (`KeyTooLarge`/`ValueTooLarge`/`CasMismatch{expected, actual}`/`Backend`), `NoopNodeStateStore` + static `NOOP_NODE_STATE_STORE`.
- Extended `NodeCtx` with `pub state: &'a dyn NodeStateStore`; updated every existing `NodeCtx::new` call site across `starter-flow*`, `starter-insights`, `starter-ext-flow` to thread it (Noop in tests/built-ins; engine threads a real `Arc<dyn NodeStateStore>` via the propagator).
- `starter-flow::state::in_memory::InMemoryNodeStateStore` over `Arc<RwLock<HashMap<NodeStateKey,(Vec<u8>,u64)>>>`.
- `starter-store-sqlite::flow::node_state::SqliteNodeStateStore` + `migrations/flow/0003_node_state.sql` (PK `(flow_id,node_id,key)`, `version INTEGER`, `BEGIN IMMEDIATE` mutations).
- Parameterised matrix tests in both crates (`get-missing`, `get-after-put`, `put-overwrites`, `cas-success`, `cas-mismatch`, `cas-initial-requires-zero-expected`, `delete-then-get-missing`) — all green.
- DOCS/flow/scope/node-state.md (~200 lines): R5 reconciliation, keying scheme, CAS semantics, two-impl pattern, size caps, `reset_on_redeploy` semantics, Noop fallback.
- Threaded `node_state: Arc<dyn NodeStateStore>` through `propagator::{spawn,spawn_with_checkpoint,drive,drive_with_checkpoint}` and `run.rs` (currently Noop-defaulted; rubix wiring lands in a later stage).
- Committed: `291d2fd` on branch `codeless/rubix-flow-live-tick-demo`.

## Next

- (none — fresh session picks up the next stage from the job script)

## What you need to know

- `starter_flow_spi_baseline_holds` in `crates/starter-flow/tests/workspace_dep_tree_gates.rs` is failing — confirmed pre-existing (drift is `serde_json 1.0.149→1.0.150` + autocfg build-deps in the lockfile; reproduces on the parent commit when untracked stash is included). Stage-required `cargo test -p starter-flow-spi -p starter-flow -p starter-store-sqlite` is otherwise fully green; I ran it with `--skip starter_flow_spi_baseline_holds` to confirm.
- `starter-store-sqlite` tests require `--features flow,testing`.
- Workspace-wide `cargo build` fails because the locked AWS SDK crates need rustc 1.91 and the toolchain here is 1.90 — also pre-existing, not from this stage.
- The propagator's `node_state` argument defaults to `NoopNodeStateStore` from `FlowRunner::start_run`; the planned rubix-side wiring swaps in `Arc<SqliteNodeStateStore>`.

## Open questions

- (none)
