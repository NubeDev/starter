## Done

- Implemented `ToolCall` `NodeBehavior` in `crates/starter-flow-nodes/src/tool_call.rs` behind the `tool-call` feature: validates `tool_id` as a reverse-DNS `KindId`, reads `input` JSON, resolves via `Arc<dyn ToolRegistry>`, writes result to `output` slot, races against `NodeCtx::cancel` via `tokio::select!`, and emits a `tool_call.invoke` tracing span (`node_id`, `tool_id`, `principal_id_hash`, `cancel_observed`).
- Added `ToolRegistry` trait + `StaticToolRegistry` host-built impl; typed `ToolCallError` (MissingToolId / InvalidToolId / MissingInput / UnregisteredTool / ToolFailed).
- 6 unit tests pass under `cargo test -p starter-flow-nodes --features tool-call`: happy path, typed tool error, cancel-mid-call within 200 ms returning `NodeError::Cancelled`, unregistered tool, missing tool_id, invalid (non-reverse-DNS) tool_id. Tests use a `MockTool` that records invocation arguments. Clippy + fmt green; workspace check green.
- Cargo: added `starter-spi` and `serde_json` as unconditional deps and `tokio` (macros/sync/rt) as `optional`, pulled in via `tool-call = ["dep:tokio"]`.
- Committed as `ab86450` on branch `codeless/starter-flow-engine-finish`.

## Next

- Stage 5: the two SCOPE Phase 2 Smoke tests ("One write chokepoint" and "Engine is reader of policies") + the R3 grep-contract test.

## What you need to know

- `NodeCtx` still does not thread `Principal` or `EventSink` — the SPI `NodeCtx` is `#[non_exhaustive]` but only carries `run`, `node`, `cancel`. The brief mentions Principal/EventSink threading; I recorded `principal_id_hash` as `tracing::field::Empty` and noted the adapter records principal in its enclosing span. If a later stage adds Principal to `NodeCtx`, fill the span field then.
- `starter_spi::Tool::invoke(&self, serde_json::Value) -> Result<Value>` — no Cancel parameter. Cancellation is implemented by racing the tool future against `ctx.cancel.cancelled()` in `tokio::select!`; the in-flight future is dropped on cancel.
- Tool input/output cross the slot boundary as `SlotValue::Json(serde_json::Value)`. Slot constants exported: `TOOL_ID_SLOT="tool_id"`, `TOOL_INPUT_SLOT="input"`, `TOOL_OUTPUT_SLOT="output"`.
- Cancellation test relies on real time (`tokio::time::sleep(30s)`) + 200 ms timeout — no `start_paused`.

## Open questions

- Should `NodeCtx` be extended in a future stage to carry `Principal` + `EventSink` so the `principal_id_hash` span field is non-empty? Out of scope here but the brief implies yes.
