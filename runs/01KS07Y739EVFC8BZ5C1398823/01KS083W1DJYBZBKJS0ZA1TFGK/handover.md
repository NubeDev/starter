## Done

- Implemented `Transform` `NodeBehavior` in `crates/starter-flow-nodes/src/transform.rs` behind the `transform` feature: `&self` (R5), pulls `fn_id` (`SlotValue::String`) from the input `SlotMap`, resolves it against a host-built `TransformFunctionRegistry`, applies the closure, returns the output `SlotMap` so the propagator writes via the single `GraphStore::write_slot` chokepoint (R2). `KIND_ID = "starter.flow.transform"` verbatim.
- Exposed public seam: `TransformFunctionRegistry` trait + `StaticTransformFunctionRegistry` impl, plus `TransformFn = Arc<dyn Fn(SlotMap) -> SlotMap + Send + Sync + 'static>` and `FN_ID_SLOT` constant.
- Added typed `TransformError` (`MissingFnId`, `UnregisteredFn(String)`, `Panicked { fn_id, message }`) surfaced as `NodeError::Other`; propagator turns these into `FlowEvent::NodeFailed`. Panic catch via `std::panic::catch_unwind(AssertUnwindSafe(...))` so a bad closure never crashes the propagator task.
- R12 span: `tracing::info_span!("transform.invoke", node_id, fn_id, input_kind, output_kind)` with `output_kind` recorded via `Span::record` after invoke.
- Added 5 unit tests (identity, sum, panicking, unregistered, missing fn_id) and integration test `crates/starter-flow-nodes/tests/transform_node_failed.rs` (2 cases) driving the propagator end-to-end to assert `FlowEvent::NodeFailed`.
- Updated `crates/starter-flow-nodes/Cargo.toml`: added `async-trait`, `thiserror`, `tracing` deps; dev-deps `tokio` (macros/time) and `starter-flow`.
- Greens: `cargo test -p starter-flow-nodes --features transform` (5 unit + 2 integration), `cargo clippy --all-targets -- -D warnings`, `cargo fmt`.
- Committed as `ed6f202` on branch `codeless/starter-flow-engine-finish`.

## Next

- Stage 4: tool_call node-kind body. Per stage 1 lock, look up the `Tool` via an `Arc<dyn ToolRegistry>` threaded through the run (not a global static). KIND_ID `starter.flow.tool_call` should stay verbatim.

## What you need to know

- `NodeBehavior::invoke` returns `Result<SlotMap, NodeError>`; the propagator writes outputs via `GraphStore::write_slot` and converts `Err` to `FlowEvent::NodeFailed`. The transform body therefore never calls `write_slot` directly — R2 chokepoint preserved by simply returning the output map.
- `RunSpec` in `starter-flow::run` is `#[non_exhaustive]`, so the integration test drives `propagator::spawn` directly with a hand-built `FlowTopology` + `RunCancel`. Stage 4 will likely want the same pattern, or to add a `RunSpec::new` if engine-side changes become unavoidable (stage brief forbids engine refactors).
- `TransformFn` is **synchronous** (`Fn(SlotMap) -> SlotMap`), keeping `catch_unwind` viable. If stage 4 needs an async tool call (which it will), it won't be able to reuse the same panic-catch shape — it'll need `futures::FutureExt::catch_unwind` with `AssertUnwindSafe`.
- The fn_id slot lives in the same input `SlotMap` as the payload (under the well-known name `FN_ID_SLOT = "fn_id"`). Topology authors must include `"fn_id"` in the node's trigger slots if they want fn_id changes to fire the node.
- `SlotValue` is `#[non_exhaustive]` — the helper `slot_value_kind` has a wildcard arm returning `"unknown"` for forward-compatibility.

## Open questions

- (none)
