//! Convert one parsed [`RubixFlowYaml`] into the typed starter-flow
//! body triple the host's `FlowRegistry` accepts.

use starter_flow::definition::body::{FlowBody, LinkDecl, NodeDecl};
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, NodeId};

use crate::error::LoadError;
use crate::yaml::{RubixFlowYaml, ALLOWED_TOOLS_KEY};

/// Surface string contributors type in YAML (`kind: ai-agent`).
pub const AI_AGENT_KIND_YAML: &str = "ai-agent";

/// Registered reverse-DNS [`KindId`] the surface string maps to.
pub const AI_AGENT_KIND_ID: &str = "com.rubix.ai-agent";

/// Prefix prepended to short YAML node ids to produce reverse-DNS [`NodeId`]s.
pub const NODE_ID_PREFIX: &str = "com.rubix";

/// Default seed slot used by the host's default adapter pair. Also
/// added to the root node's `triggers` so the engine fires on seed.
pub const DEFAULT_SEED_SLOT: &str = "payload";

/// Default terminal slot the default output adapter reads.
pub const DEFAULT_OUTPUT_SLOT: &str = "out";

/// Convert one parsed yaml into `(FlowId, FlowRevisionId, FlowBody)`.
/// A fresh [`FlowRevisionId`] is minted per call.
pub fn convert(
    path: &str,
    yaml: RubixFlowYaml,
) -> Result<(FlowId, FlowRevisionId, FlowBody), LoadError> {
    let flow_id = FlowId::new(yaml.id.clone()).map_err(|e| LoadError::Id {
        path: path.to_owned(),
        id: yaml.id.clone(),
        source: e,
    })?;
    if yaml.nodes.is_empty() {
        return Err(LoadError::EmptyBody {
            path: path.to_owned(),
        });
    }

    let mut body = FlowBody::new(flow_id.clone());
    for node in yaml.nodes {
        let qid = format!("{NODE_ID_PREFIX}.{}", node.id);
        let node_id = NodeId::new(qid.clone()).map_err(|e| LoadError::Id {
            path: path.to_owned(),
            id: qid,
            source: e,
        })?;
        let kind_str = if node.kind == AI_AGENT_KIND_YAML {
            AI_AGENT_KIND_ID.to_owned()
        } else {
            node.kind.clone()
        };
        let kind = KindId::new(kind_str.clone()).map_err(|e| LoadError::Id {
            path: path.to_owned(),
            id: kind_str,
            source: e,
        })?;
        // Pass the YAML config bag through verbatim, then *explicitly*
        // overwrite `allowed_tools` with the typed, validated, full
        // list. This is the seam that lets AgentLoop's `ToolSet`
        // filter scope per flow — every entry in the YAML list ends
        // up on the AiAgentNode config, not just `[0]`.
        let mut settings = serde_json::to_value(&node.config)
            .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));
        let allowed = node.allowed_tools()?;
        if !allowed.is_empty() {
            let obj = match settings {
                serde_json::Value::Object(m) => m,
                _ => serde_json::Map::new(),
            };
            let mut obj = obj;
            obj.insert(
                ALLOWED_TOOLS_KEY.to_owned(),
                serde_json::Value::Array(
                    allowed
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect(),
                ),
            );
            settings = serde_json::Value::Object(obj);
        }
        let mut decl = NodeDecl::new(node_id, kind);
        decl.settings = settings;
        decl.triggers = vec![DEFAULT_SEED_SLOT.to_owned()];
        body.nodes.push(decl);
    }
    body.links = yaml
        .links
        .into_iter()
        .map(|l| LinkDecl::new(l.from, l.to))
        .collect();
    Ok((flow_id, FlowRevisionId::new(), body))
}
