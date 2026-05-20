## Done

- Stage 5 agent-as-tool bridge landed in commit `331d1bb`.
- `AiRuntime` now holds `flows`, `engine`, `runs`, `hub`; new ctor `AiRuntime::new(flows, engine, runs, hub)` and `with_registry(registry, …)` for test injection.
- `AiRuntime::synthesize_flow_tools(agent_tools)` builds `ToolDef`s from `FlowStore::list()` filtered by `flow:*` / `flow:<id>` entries; permissive `{type:"object", additionalProperties:true}` schema per SCOPE.
- `AiRuntime::invoke_flow_tool(flow_id, payload)` fires the flow via `FlowEngine`, persists a `runs` row, drains engine events onto `EventHub.runs`, and returns the terminal `RunCompleted.output` map as JSON.
- `drive_chat` (in `agent_bridge.rs`) runs the multi-turn REST loop: per turn, hands the runner the synthesised tools, intercepts `flow:*` ToolUse, dispatches via the bridge, surfaces `tool-call`/`tool-result` SSE frames, and feeds the reply back as a `user`-role history message. CLI-shape providers (Claude/Codex/Copilot) keep the existing single-shot path.
- New `run_agent_raw` returns the canonical `data:` payload strings; `run_agent` wraps each in `sse::Event` and is unchanged at the REST surface.
- Added `FlowEngine::with_quiescence(d)` so tests can shorten the 60 s engine quiescence (production wiring keeps 60 s).
- Bridge code split across `agent_bridge.rs` (391 lines) + `run_drain.rs` (128 lines); `ai_runtime.rs` trimmed to 279 lines. All three under the workspace 400-line rule.
- Integration test `examples/flow-agent/tests/agent_tool_bridge.rs` runs the full path against a `RecordingAiRunner`: trigger→log flow + agent with `tools=["flow:<id>"]`, asserts (a) turn-0 advertises the synthesised tool, (b) `tool-call`+`tool-result` SSE frames surface, (c) the log's `.emitted` slot value reaches the agent on turn-1 history, (d) a `runs` row is recorded.
- `cargo test -p flow-agent` green.

## Next

- (none) — stage 6 is the next session's job (per SCOPE phase 6: nested sidebar + persistence per F7 / SIDEBAR.md).

## What you need to know

- The bridge only fires the agentic loop when the agent's provider is REST-shape (Anthropic / OpenAI) AND the `tools` array declares at least one `flow:*` / `flow:<id>` entry. CLI runners (Claude CLI) hit the existing single-shot path because the CLI wrapper owns its own tool loop.
- Engine `RunCompleted.output` keys terminal slots as `"{engine_node_id}.{slot_name}"`. The bridge's tool-result payload is `{ "flow-agent.nodes.<ui_id>.<slot>": <value>, … }`. The test asserts on the `.emitted` suffix rather than a fixed key for that reason.
- `with_quiescence` was needed because the production default (60 s, sized for slow Claude CLI calls) would blow past test timeouts on cheap `trigger → log` flows. The test sets 200 ms.
- File-split rationale: `ai_runtime.rs` owns the public surface + registry + chat orchestration; `agent_bridge.rs` owns the bridge `impl AiRuntime` block (synthesize / dispatch / drive_chat); `run_drain.rs` owns the engine-event pump + slot → JSON converter. Each is under 400 lines.
- The recording runner used in the test ignores its inputs (it just replays a scripted turn list); the loop's contract is validated through `RecordedCall::history_len` and `tools_count`, plus the SSE frame stream.

## Open questions

- (none) — scope items for stage 5 all addressed; manual smoke (real Anthropic key + UI agent) is left to the next stage's setup, since stage 5 explicitly says "manual smoke" rather than "automated smoke".
