# AI PROVIDERS — how `rubix-agent` selects an `AiRunner`

## The model

`rubix-agent` runs every bundled flow through starter's `ai-agent`
node kind. That node holds one `Arc<dyn AiRunner>` (see
[`starter_spi::ai::AiRunner`](../../../crates/starter-spi/src/ai/)).
Rubix builds the runner once at boot and hands the same instance to
every `ai-agent` invocation — there is no second LLM seam.

## The contract

The runner is constructed by
[`boot::ai::build_runner`](../../crates/rubix-agent/src/boot/ai.rs):

```rust
pub fn build_runner(cfg: &AgentConfig) -> Result<Arc<dyn AiRunner>, AiError>;
```

`AgentConfig::ai_provider` (loaded by
[`boot::config::AgentConfig::load`](../../crates/rubix-agent/src/boot/config.rs))
picks which concrete runner is constructed:

| `ai_provider` | Resolved runner | Status |
|---|---|---|
| `claude-cli` (default) | [`starter_ai::runners::claude::ClaudeRunner`](../../../crates/starter-ai/src/runners/claude.rs) | live |
| `anthropic` | Anthropic REST | returns `AiError::Unimplemented`; see [`crates/starter-ai-agent/LONG-TERM.md`](../../../crates/starter-ai-agent/LONG-TERM.md) |

`build_runner` is the **only** call site that constructs an
`AiRunner` in the binary. `boot::mcp::build_mcp_surface` reads the
runner from this function and registers it on the rubix `ai-agent`
node kind alongside the snapshot of `crate::registry::build_tool_registry`
the loop dispatches from.

## Default — zero-config Claude CLI

With no `ai_provider` set, the binary resolves to `ClaudeRunner`.
The `claude` binary on PATH manages its own auth (`claude login`);
rubix does not look at `ANTHROPIC_API_KEY`. The commented sample
in `rubix/dev/agent.toml` documents the default:

```toml
# ai_provider = "claude-cli"
```

## Recorded-LLM fixture (`RUBIX_AI_FIXTURE`)

When `RUBIX_AI_FIXTURE` points at a JSON-script file,
`build_runner` swaps `ClaudeRunner` for a replay runner whose
turns are fed straight off the file. Each script element is
`{ "text": "<final>", "tool_uses": [{ id, name, input }] }`:

* `tool_uses` empty → terminal turn, `text` becomes the agent reply.
* `tool_uses` non-empty → `AgentLoop` dispatches each tool against
  the host's `crate::registry::build_tool_registry` snapshot, then
  pops the next scripted turn.

This is the seam `mcp_stdio_test` uses to assert the agent loop
end-to-end without calling out to a live model. Fixtures live in
`rubix/crates/rubix-agent/tests/fixtures/`.

## What rubix does NOT do

- **No multi-provider fan-out.** One runner per binary, chosen at boot.
- **No per-skill override.** Skills steer behaviour through their
  prompt, not through provider choice.
- **No auto-fallback chain.** A missing `claude` binary surfaces as
  a runner error on first invocation, not a silent switch.
- **No auth handling.** CLI providers manage their own auth.

## Failure modes

| Failure | Surface |
|---|---|
| `ai_provider = "anthropic"` | `build_runner` returns `AiError::Unimplemented` at boot; binary exits |
| `ai_provider` unknown value | `AiError::Unknown` at boot |
| `RUBIX_AI_FIXTURE` path missing / bad JSON | `AiError::Fixture` at boot |
| `claude` binary missing on PATH | First `AiAgentNode` invocation surfaces `RunnerError` through the flow engine |

Boot-time failures stop the binary. Per-invocation failures surface
as `NodeError::Backend` from the rubix `ai-agent` kind.

## Open work (deferred to LONG-TERM.md)

- Multi-turn session persistence (`SessionStore`)
- Per-turn cost cap
- Cooperative cancellation observed inside the loop
- Tool-call streaming via the R13 SSE taxonomy
- First-class skill resolution / enforcement
- `RunnerInput::Cli` dispatch path (today `AgentLoop` builds
  `RunnerInput::Rest`; the live `ClaudeRunner` is CLI-only and
  rejects `Rest`, so the fixture runner is the only seam the loop
  drives end-to-end. The CLI-input branch is the next thing
  every Block-D extension needs).

Each is sectioned in
[`crates/starter-ai-agent/LONG-TERM.md`](../../../crates/starter-ai-agent/LONG-TERM.md).
