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
