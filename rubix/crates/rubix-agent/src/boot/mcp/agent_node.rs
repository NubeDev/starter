//! `com.rubix.ai-agent` [`NodeBehavior`] implementation.
//!
//! Thin adapter binding the kind id every bundled rubix YAML uses
//! (`com.rubix.ai-agent`) to [`starter_ai_agent::AgentLoop`]. The
//! seed adapter at [`super::register`] writes a JSON payload
//! containing `locale`, `prefs`, and the caller's MCP `arguments`
//! JSON onto the `payload` slot; this body builds a prompt from
//! that payload, drives the agent loop, and writes the reply to
//! the `out` slot the output adapter reads back.
//!
//! Behaviour is deliberately split into two paths so the
//! deterministic part of the response is independent of the
//! non-deterministic LLM round-trip:
//!
//! 1. **Primary-tool dispatch.** Each node with a mapped primary
//!    tool dispatches it; the structured `Diagnostic` IS the
//!    response. This is the smoke-test path.
//! 2. **LLM narration.** On by default; disable with
//!    `RUBIX_AI_NARRATION=0` for pure-tool responses (no LLM cost,
//!    deterministic CI). The agent loop today returns only a
//!    free-form reply (see crates/starter-ai-agent/LONG-TERM.md
//!    §"CLI runner tool dispatch"); failures do not fail the node
//!    so the deterministic tool output still reaches the caller.
//!    Long-running narration awaits no longer race the run
//!    coordinator's quiescence window — the in-flight node tracker
//!    in starter-flow's run coordinator holds completion until
//!    `NodeEmitted` arrives.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use starter_ai_agent::{AgentLoop, ToolSet};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotValue};

use starter_spi::ai::AiRunner;
use starter_spi::tool::Tool;

/// `com.rubix.ai-agent` node kind.
pub(super) struct RubixAiAgentNode {
    kind: KindId,
    runner: Arc<dyn AiRunner>,
    tools: Vec<Arc<dyn Tool>>,
    /// Per-`NodeId` primary tool name — extracted at boot from each
    /// flow YAML's `allowed_tools[0]`. A node listed here dispatches
    /// its primary tool deterministically; nodes absent from the map
    /// fall back to the agent-loop-reply path.
    primary_tools: HashMap<NodeId, String>,
}

impl RubixAiAgentNode {
    pub(super) fn new(
        kind: KindId,
        runner: Arc<dyn AiRunner>,
        tools: Vec<Arc<dyn Tool>>,
        primary_tools: HashMap<NodeId, String>,
    ) -> Self {
        Self {
            kind,
            runner,
            tools,
            primary_tools,
        }
    }

    fn find_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .iter()
            .find(|t| t.definition().name == name)
            .cloned()
    }
}

#[async_trait]
impl NodeBehavior for RubixAiAgentNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        // The seed adapter writes the locale + prefs + caller input
        // onto the `payload` slot as a JSON object. Read it back so
        // we can both (a) forward `input` to the primary tool's
        // `invoke` and (b) hand the caller context to the LLM as a
        // free-form prompt for the optional `reply` field.
        let payload = match input.get(rubix_flows::DEFAULT_SEED_SLOT) {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => json!({}),
        };
        let tool_input = payload
            .get("input")
            .cloned()
            .unwrap_or_else(|| json!({}));

        // Primary-tool dispatch — the deterministic part of the
        // output. For nodes with a mapped primary tool, the tool's
        // output IS the structured response; the LLM reply is a
        // bonus narration field that the smoke test does not assert
        // against. For nodes without a mapping, fall back to a
        // reply-only response.
        let tool_value: Option<Value> = match self.primary_tools.get(ctx.node) {
            Some(tool_name) => {
                let tool = self.find_tool(tool_name).ok_or_else(|| {
                    NodeError::Backend(format!(
                        "ai-agent: primary tool `{tool_name}` not in registry"
                    ))
                })?;
                let value = tool.invoke(tool_input).await.map_err(|e| {
                    NodeError::Backend(format!("ai-agent: tool `{tool_name}` failed: {e}"))
                })?;
                Some(value)
            }
            None => None,
        };

        // LLM narration on by default. The starter-flow run
        // coordinator now tracks in-flight nodes so a long
        // `behavior.invoke` await no longer races the quiescence
        // window — see the `slow_node_body_does_not_race_quiescence`
        // test in starter-flow. Operators who want pure tool output
        // (no LLM cost / latency, deterministic CI) can disable with
        // `RUBIX_AI_NARRATION=0`.
        //
        // Failures in the agent loop never fail the node — the
        // deterministic tool output still reaches the caller.
        let locale = payload
            .get("locale")
            .and_then(|v| v.as_str())
            .unwrap_or("en");
        let narration_enabled = std::env::var("RUBIX_AI_NARRATION")
            .map(|v| !(v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off")))
            .unwrap_or(true);
        let reply = if narration_enabled {
            let prompt = format!(
                "Summarise this rubix flow result in one sentence for the operator. \
                 Respond in BCP-47 locale `{locale}` (translate the summary into \
                 the matching language; e.g. `es-AR` → Spanish, `en-US` → English). \
                 Caller context (JSON): {payload}\n\nTool output (JSON): {}",
                tool_value
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "<none>".to_owned())
            );
            let agent = AgentLoop::new(self.runner.clone(), ToolSet::new(self.tools.clone()));
            match agent.run(prompt).await {
                Ok(reply) => Some(reply),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        node = %ctx.node,
                        "ai-agent narration failed; returning tool output only"
                    );
                    None
                }
            }
        } else {
            None
        };

        let body = match (tool_value, reply) {
            (Some(t), Some(r)) => json!({ "tool": t, "reply": r }),
            (Some(t), None) => json!({ "tool": t }),
            (None, Some(r)) => json!({ "reply": r }),
            (None, None) => {
                return Err(NodeError::Backend(
                    "ai-agent: neither primary tool nor LLM reply produced output".to_owned(),
                ));
            }
        };

        let mut out = SlotMap::new();
        out.insert(
            rubix_flows::DEFAULT_OUTPUT_SLOT.to_owned(),
            SlotValue::Json(body),
        );
        Ok(out)
    }
}
