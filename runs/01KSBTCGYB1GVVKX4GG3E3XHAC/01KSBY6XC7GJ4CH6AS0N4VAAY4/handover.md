## Done

- Added `rubix/crates/rubix-agent/src/boot/ai.rs` with `build_runner` (default ClaudeRunner, anthropic→`Unimplemented`, env-driven `RUBIX_AI_FIXTURE` JSON-script replay runner) + `AiError` enum.
- Extended `boot::config::AgentConfig` with `ai_provider: Option<String>` (defaults to `"claude-cli"`); commented sample in `rubix/dev/agent.toml`.
- Replaced the Block A `AiAgentStubNode` in `boot::mcp` with `RubixAiAgentNode`, wrapping `starter_ai_agent::AgentLoop` over `crate::registry::build_tool_registry(None)`. Registered under `com.rubix.ai-agent` (matches `rubix_flows::AI_AGENT_KIND_ID`). Boot log emits `node_kinds=com.rubix.ai-agent`.
- Added rubix-agent deps on `starter-ai-agent`, `starter-flow-node-loop`, `starter-ai` (feature `provider-claude`).
- Recorded-LLM fixtures: `tests/fixtures/scheduled-system-check-{en,es}.json` (two-turn: tool_use → final text).
- Rewrote `tests/mcp_stdio_test.rs` to use `RUBIX_AI_FIXTURE`, assert ≥6 tools, structural disk shape (`code` prefix + numeric percent/free); all three default-enabled tests pass; one DB-backed test stays `#[ignore]`d.
- Rewrote `docs/design/ai-providers/README.md` (present-tense `build_runner` contract + fixture seam) and added a "live agent path" section to `docs/design/agent/README.md`.
- `cargo build -p rubix-agent` green, `cargo test -p rubix-agent` 16/0 + 6/0 + 3/0 + 4/0 + 4/0 green, `./rubix/scripts/lint-doc-refs.sh` clean.
- Committed as `f8a1f17` on `codeless/rubix-agent-runtime`.

## Next

- (none — stage 6 picks up next)

## What you need to know

- The stage spec referenced `tests/mcp_disk_test.rs`; no such file exists in the tree (only `cli_disk_test.rs` + `rest_disk_test.rs` + `mcp_stdio_test.rs`). I therefore put the recorded-LLM fixture treatment on `mcp_stdio_test.rs` only and noted the absent file here rather than fabricating one.
- The spec text said `AiAgentNode::KIND_ID = "ai-agent"`, but the actual `starter_flow_node_loop::KIND_ID` is `"com.starter.ai-agent"`, while the bundled rubix flows convert their `kind: ai-agent` to `"com.rubix.ai-agent"`. Per stage's wiring intent (bundled flows must resolve), the rubix-agent registers a thin local `RubixAiAgentNode` wrapper under `"com.rubix.ai-agent"` rather than registering the upstream `AiAgentNode` directly. The wrapper delegates to `AgentLoop::new(runner, ToolSet::new(tools)).run(prompt)` and writes `{reply, code, params}` JSON to the `out` slot.
- `AgentLoop` always sends `RunnerInput::Rest`, but `ClaudeRunner` only accepts CLI input — so the live Claude path will currently error on first invocation. The fixture runner is the only seam that drives the loop end-to-end in CI today. This gap is documented in `docs/design/ai-providers/README.md` "Open work" and remains in the LONG-TERM bucket — fixing it requires either a CLI dispatch path in `AgentLoop` (touches stage 2 code) or adapting `ClaudeRunner` to accept Rest input.
- The prompt v0 is a one-line framing string wrapping the seed payload JSON. Skill-aware prompt construction is deferred per scope.

## Open questions

- Should the missing `mcp_disk_test.rs` be authored from scratch in a follow-up, or fold the disk-shape assertions into `mcp_stdio_test.rs` permanently (current state)?
- The `AgentLoop` ↔ `ClaudeRunner` input-kind mismatch: fix in `starter-ai-agent` (Block B revision) or in `starter-ai`'s ClaudeRunner (loosen to also accept `RunnerInput::Rest`)?
