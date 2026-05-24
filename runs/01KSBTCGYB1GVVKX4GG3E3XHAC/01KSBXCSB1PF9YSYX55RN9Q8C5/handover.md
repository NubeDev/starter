## Done

- Implemented Block B upstream crates: `crates/starter-ai-agent/` (AgentLoop, ToolSet, AgentError, prompt helpers, MockAiRunner, LONG-TERM.md) and `crates/starter-flow-node-loop/` (AiAgentNode wrapping AgentLoop under KIND_ID `com.starter.ai-agent`).
- All four integration tests pass: `single_turn_no_tools_test`, `single_turn_with_tools_test`, `unknown_tool_test`, `invoke_test`. `cargo build -p starter-ai-agent -p starter-flow-node-loop` clean; targeted clippy clean.
- Both crates registered in root `Cargo.toml` `[workspace.members]` and `[workspace.dependencies]`.
- `rubix/docs/design/starter-changes/phase-1.md` flipped the `starter-flow-node-loop` entry to **landed (in-tree)** and added a new `starter-ai-agent` entry; README.md phase summary updated.
- Durable stage-1 fixes (operator policy on the prior auto-bypass): refactored `rubix/crates/rubix-flows/src/load.rs` from 384 lines into four verb files (error.rs, yaml.rs, convert.rs, load.rs — each ≤80 lines) with tests moved to `tests/load_test.rs`; rewrote the "Block-A scope note" in `rubix/docs/design/flows/README.md` to drop the forbidden "stub" wording.
- Committed as `46be9c4` (`stage 2 (block B, starter upstream — starter-ai-agent + starter-flow-node-loop)`).

## Next

- Stage 3 REVIEW gate for Block B.
- Then Block C: rubix-side `boot::ai::build_runner` returning `Arc<dyn AiRunner>` via `starter_ai::runners::claude::ClaudeRunner`; replace the rubix `AiAgentStubNode` in `rubix/crates/rubix-agent/src/boot/mcp.rs` with `starter_flow_node_loop::AiAgentNode` and re-map the rubix YAML `ai-agent` kind to the upstream `KIND_ID = "com.starter.ai-agent"` (today rubix-flows still maps it to `com.rubix.ai-agent`).

## What you need to know

- KIND_ID had to be reverse-DNS (`com.starter.ai-agent`) — the literal `"ai-agent"` from the SCOPE sketch failed `KindId::new` validation. Block C must adjust `rubix-flows::AI_AGENT_KIND_ID` or the kind registration in `boot/mcp.rs` so the upstream KIND_ID is what's bound under the registry. The simplest path: replace `rubix_flows::AI_AGENT_KIND_ID` constant with `starter_flow_node_loop::KIND_ID` and have the loader map the YAML surface string `ai-agent` to that.
- `AgentError` is `#[non_exhaustive]`; `starter-flow-node-loop`'s `map_err` has a wildcard arm so future variants (CostCapHit, Cancelled, SkillViolation per LONG-TERM.md) lower to `NodeError::Backend` until a richer mapping lands.
- Workspace-wide `cargo clippy --workspace` is blocked by a pre-existing `aws-smithy-*` MSRV mismatch (rustc 1.90 vs required 1.91); not introduced by this stage. The two new crates clippy-clean individually with `-D warnings`.
- The starter-spi `ai::input`/`ai::result`/`ai::session` modules are private; use the `starter_spi::ai::*` re-exports.
- `MockAiRunner` lives in `starter_ai_agent::testing` (public, not cfg(test)-gated) so downstream crates' tests can use it; `starter-flow-node-loop/tests/invoke_test.rs` already does.

## Open questions

- (none)
