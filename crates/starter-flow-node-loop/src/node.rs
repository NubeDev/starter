//! [`AiAgentNode`] — the `ai-agent` [`NodeBehavior`].

use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use starter_ai_agent::{AgentError, AgentLoop, ToolSet};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::ai::AiRunner;
use starter_spi::tool::Tool;

/// Reverse-DNS kind id every `ai-agent` node registers under. The
/// short `ai-agent` label is the human-facing YAML surface; the
/// registered id has to satisfy [`KindId`]'s reverse-DNS rule.
pub const KIND_ID: &str = "com.starter.ai-agent";

/// Input slot the node reads the user prompt from.
pub const IN_SLOT_PROMPT: &str = "prompt";

/// Output slot the model's final reply lands on.
pub const OUT_SLOT_REPLY: &str = "out";

/// Optional settings key naming the per-invocation tool allow-list.
/// The value is parsed as a JSON array of strings; when absent the
/// node exposes its full bound tool set.
pub const SETTINGS_ALLOWED_TOOLS: &str = "allowed_tools";

/// Flow-node wrapper around an [`AgentLoop`].
pub struct AiAgentNode {
    runner: Arc<dyn AiRunner>,
    tools: Vec<Arc<dyn Tool>>,
    kind: OnceLock<KindId>,
}

impl AiAgentNode {
    /// Build a node bound to one runner and the host's full tool set.
    /// Per-invocation filtering happens against the node's
    /// `allowed_tools` setting.
    pub fn new(runner: Arc<dyn AiRunner>, tools: Vec<Arc<dyn Tool>>) -> Self {
        Self {
            runner,
            tools,
            kind: OnceLock::new(),
        }
    }

    fn kind(&self) -> &KindId {
        self.kind.get_or_init(|| {
            KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS kind id")
        })
    }

    fn prompt_from(input: &SlotMap) -> Result<String, NodeError> {
        match input.get(IN_SLOT_PROMPT) {
            Some(SlotValue::String(s)) => Ok(s.clone()),
            Some(SlotValue::Json(serde_json::Value::String(s))) => Ok(s.clone()),
            Some(SlotValue::Json(v)) => Ok(v.to_string()),
            Some(_) => Err(NodeError::InvalidInput(format!(
                "`{IN_SLOT_PROMPT}` slot must be a string"
            ))),
            None => Err(NodeError::InvalidInput(format!(
                "`{IN_SLOT_PROMPT}` slot is required"
            ))),
        }
    }

    fn filtered_tools(&self, input: &SlotMap) -> Vec<Arc<dyn Tool>> {
        let Some(value) = input.get(SETTINGS_ALLOWED_TOOLS) else {
            return self.tools.clone();
        };
        let allow: Option<HashSet<String>> = match value {
            SlotValue::Json(serde_json::Value::Array(arr)) => Some(
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect(),
            ),
            _ => None,
        };
        match allow {
            Some(set) => self
                .tools
                .iter()
                .filter(|t| set.contains(&t.definition().name))
                .cloned()
                .collect(),
            None => self.tools.clone(),
        }
    }
}

#[async_trait]
impl NodeBehavior for AiAgentNode {
    fn kind_id(&self) -> &KindId {
        self.kind()
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let prompt = Self::prompt_from(&input)?;
        let tools = ToolSet::new(self.filtered_tools(&input));
        let agent = AgentLoop::new(self.runner.clone(), tools);
        let reply = agent.run(prompt).await.map_err(map_err)?;

        let mut out = SlotMap::new();
        out.insert(OUT_SLOT_REPLY.to_owned(), SlotValue::String(reply));
        Ok(out)
    }
}

fn map_err(e: AgentError) -> NodeError {
    match e {
        AgentError::UnknownTool(name) => NodeError::Domain {
            code: "unknown_tool",
            message: format!("model asked for unregistered tool `{name}`"),
        },
        AgentError::Runner(s) | AgentError::Unparseable(s) => NodeError::Backend(s),
        AgentError::Tool { name, message } => {
            NodeError::Backend(format!("tool `{name}`: {message}"))
        }
        // `AgentError` is `#[non_exhaustive]`; surface anything new
        // long-term work adds (cost cap, cancel, skill violation) as
        // a backend failure until a richer mapping is defined.
        other => NodeError::Backend(other.to_string()),
    }
}
