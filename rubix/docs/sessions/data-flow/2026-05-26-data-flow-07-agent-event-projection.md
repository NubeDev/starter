# Follow-up — project ai-agent inner events onto the flow event bus

Spun out of stage 07. The original stage-07 success bar asserted
"no `ToolUse` event with name `Bash` / `Read` / `Write` / `Edit`"
against the JSON response from
`POST /api/v1/flows/{id}/run`. That assertion presupposes a wire
shape (`{events: [{kind: "ToolUse", name: "…"}, …]}`) that does not
exist today: the `Text` / `ToolUse` / `ToolResult` events live
inside the wrapped Claude CLI run, surfaced by
[`AgentLoop::run`](../../../crates/starter-ai-agent/src/agent_loop.rs)
only as a single concatenated `text` reply, and the
`ai-agent` node body
([`boot/mcp/agent_node.rs`](../../../crates/rubix-agent/src/boot/mcp/agent_node.rs))
writes that reply onto a terminal slot without projecting the
per-step events onto the engine's [`FlowEvent`] bus.

Until that gap is bridged, stage 07's "Bash didn't fire" check is
defensive-by-construction (CLI built-in surface locked down via
`tools: []` → `CliCfg::tools = Some("")` so the model has no Bash
to reach for), confirmed via the structured self-check log line in
[`agent_node.rs`](../../../crates/rubix-agent/src/boot/mcp/agent_node.rs)
(target `rubix.ai_agent.self_check`).

## What "done" looks like

A `POST /api/v1/flows/{id}/run` response carries an `events` array
of one entry per CLI stream event observed during the agent's
inner run:

```json
{
  "flow_id": "com.rubix.dashboard-assistant",
  "output": { "reply": "…", "primary_tool_output": { … } },
  "events": [
    { "kind": "Text", "content": "BANANA\nI'll create a page…" },
    { "kind": "ToolUse", "name": "mcp__rubix__rubix_dashboard_create",
      "input": { "page_id": "dashboard.disk-overview", … } },
    { "kind": "ToolResult", "name": "mcp__rubix__rubix_dashboard_create",
      "output": { "revision_id": "…" } }
  ]
}
```

`kind` enum mirrors the `EventKind` already in
[`starter-spi/src/ai`](../../../../crates/starter-spi/src/ai/) (the
runner emits these on the channel `AgentLoop` constructs at
[agent_loop.rs:181](../../../../crates/starter-ai-agent/src/agent_loop.rs#L181)
but the loop drops them — the receiver is held by `_rx`).

## Sketch of the work

1. **`AgentLoop::run` returns events.** Change the signature from
   `Result<String, AgentError>` to
   `Result<(String, Vec<Event>), AgentError>`. The channel
   already exists; the receiver is just discarded today
   ([agent_loop.rs:181](../../../../crates/starter-ai-agent/src/agent_loop.rs#L181)).
   Drain it into a `Vec<Event>` before returning.

2. **`RubixAiAgentNode` writes events onto a new terminal slot.**
   In [agent_node.rs](../../../crates/rubix-agent/src/boot/mcp/agent_node.rs)
   the `agent.run(prompt).await` call site stitches `reply` into
   the terminal `out` slot. Add an `events` slot (or fold them
   into a JSON object on the existing `out` slot — TBD; the latter
   is cheaper because no flow YAML refers to the slot by name).

3. **Project onto `FlowEvent`?** Optional. Two paths:
   - **Don't:** keep events in the terminal-slot JSON only;
     `FlowAsTool::invoke` already returns the terminal JSON and the
     REST `/run` route passes it through. Simplest, no new event
     variants.
   - **Do:** add a `FlowEvent::AgentStep { kind, name, … }` variant
     and emit through the `FlowEventSink` so SSE
     `/api/v1/flows/{id}/events` subscribers see per-step events in
     real time. Bigger change; pays off if a UI surface ever wants
     to render the agent's thinking live.

   Recommend "don't" for the first iteration — the JSON surface is
   enough to satisfy stage 07's success bar.

4. **REST handler change.** `routes::flow_run` already returns
   `output` verbatim; once the events land in that JSON it works
   without changes. (If we go with a separate top-level `events`
   key on the response, add a one-line extraction in the handler.)

## Why this is its own stage

Touching `AgentLoop::run`'s signature is a public-surface change;
every caller (tests in
`crates/starter-flow-nodes/tests/stage7_ai_agent_*` plus
`crates/starter-ai-agent/tests/`, plus the chat_stream route) has
to be updated. Worth doing carefully and in isolation — not
bundling into stage 07's already-broad scope.

## Decisions taken

- **Stage 07 ships without this.** The CLI built-in surface
  lockdown is already enforced by `tools: []` plumbing; the
  self-check log line confirms it at runtime. The "events array"
  wire shape is nice-to-have, not required for stage 07's bar to
  be meaningful.
- **The REST `/api/v1/flows/{id}/run` response shape stays
  `{flow_id, output}` until this follow-up lands.** Adding
  `events: []` later is purely additive on the JSON.
