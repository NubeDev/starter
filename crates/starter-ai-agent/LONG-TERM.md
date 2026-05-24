# starter-ai-agent — long-term

The v0 [`AgentLoop`](src/agent_loop.rs) ships the minimum needed to
make a real LLM-driven flow node useful: one user prompt in, optional
round of tool dispatch, one final reply out. Everything below is
deliberately deferred — the contract is named, the API entry point is
named, and the failure modes are named, so the follow-on jobs can
extend the loop without re-litigating the shape.

## Multi-turn session persistence

Today every `AgentLoop::run` call is a fresh conversation. A future
constructor extension lets the caller bind an external store:

```rust
let agent = AgentLoop::new(runner, tools)
    .with_session_store(Arc::new(store), session_id);
```

The store implements a small trait that round-trips a per-turn
checkpoint blob:

```rust
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn load(&self, id: &SessionId) -> Result<Option<SessionCheckpoint>, _>;
    async fn save(&self, id: &SessionId, checkpoint: &SessionCheckpoint) -> Result<(), _>;
}

pub struct SessionCheckpoint {
    pub history: Vec<HistoryMessage>,
    pub tool_call_log: Vec<ToolCallEntry>,
    pub spend_usd: f64,
}
```

`run` loads the checkpoint before its first runner call and saves an
updated one before returning.

## Cost cap

A per-call dollar ceiling, enforced *between* runner turns so a
single-turn overrun is caught after it lands rather than left
mid-run:

```rust
let agent = AgentLoop::new(runner, tools).with_cost_cap_usd(0.50);
```

After each runner call the loop sums `RunResult::cost_usd` against
the cap; on overflow it returns `AgentError::CostCapHit { spent_usd,
cap_usd }`. The variant is added with `#[non_exhaustive]` so it lands
non-breakingly.

## Cooperative cancellation

The v0 loop wires `NoopCancel` because the only signal callers carry
today is the outer `tokio::time::timeout`. A future shape accepts
a borrowed [`starter_spi::ai::Cancel`] and observes it in two places:

- Before each runner call (skip the call entirely if already cancelled).
- Inside the per-tool dispatch loop (skip remaining tools and return
  early).

The error variant is `AgentError::Cancelled`; the trait signature
stays the same.

## Tool-call streaming

The v0 loop returns only the final reply text. The long-term shape
exposes the model's intermediate work as a stream that matches the
rubix-spi R13 SSE event taxonomy:

```rust
pub enum AgentEvent {
    Thinking(String),
    ToolStart { name: String, input: serde_json::Value },
    ToolComplete { name: String, output: serde_json::Value, duration_ms: u64 },
    FinalText(String),
}
```

Surfaced via either an `mpsc::Sender<AgentEvent>` argument to a new
`run_streaming` method, or a returned `Stream<Item = AgentEvent>`. The
choice depends on whether the consumer needs back-pressure (the
channel shape gives it for free).

## CLI runner tool dispatch (via MCP bridge)

The v0 `AgentLoop::call` dispatches on the runner's `Provider`: REST
runners (Anthropic, OpenAi) receive the full `RestCfg` with `tools`
and `history`; CLI runners (Claude, Codex, Copilot) receive only a
`CliCfg { prompt }` because `CliCfg` has no native tool-definition
field. Multi-turn history is folded into the prompt with `[role]`
prefixes.

Net effect today: a CLI-backed loop returns the model's free-form
reply but the model cannot invoke the host's `ToolSet` directly — the
`first.tool_uses` vector is always empty for CLI providers, so the
second-round tool-result stitch never runs.

The long-term shape lets the loop spin up a temporary MCP server
exposing the `ToolSet` and pass its URL + a one-shot bearer into
`CliCfg::mcp_url` / `mcp_token` (or `mcp_config_path`), so the CLI
binary calls the host's tools as normal MCP tools. The loop then
correlates the MCP tool-call events back into the same `tool_uses`
shape the REST path produces. Pieces needed:

- `starter-mcp` exposing a "one-shot stdio server" constructor that
  serves a supplied `ToolSet` and emits structured tool-call events
  on a channel.
- `AgentLoop::call` lazily starts that server for the duration of a
  CLI run, drains its event channel into `first.tool_uses` /
  `tool_results`, and shuts it down after the second runner call.

## Skill enforcement

Today the caller does the filtering: it intersects the host's full
tool registry with the skill's allow-list before constructing the
`ToolSet`. The long-term shape makes skills first-class so the loop
itself can enforce the contract:

```rust
let agent = AgentLoop::new(runner, tools).with_skill(skill_id);
```

The loop resolves the skill against a `SkillRegistry`, applies the
intersection itself, and reports policy denials as
`AgentError::SkillViolation { skill, tool }`. The caller no longer
risks shipping an over-broad `ToolSet` by mistake.
