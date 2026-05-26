//! Convert one parsed [`RubixFlowYaml`] into the typed starter-flow
//! body triple the host's `FlowRegistry` accepts.

use starter_flow::definition::body::{FlowBody, LinkDecl, NodeDecl};
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, NodeId};

use crate::error::LoadError;
use crate::yaml::{RubixFlowYaml, ALLOWED_TOOLS_KEY, TOOLS_KEY};

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
        let cli_tools = node.tools()?;
        if !allowed.is_empty() || cli_tools.is_some() {
            let obj = match settings {
                serde_json::Value::Object(m) => m,
                _ => serde_json::Map::new(),
            };
            let mut obj = obj;
            if !allowed.is_empty() {
                obj.insert(
                    ALLOWED_TOOLS_KEY.to_owned(),
                    serde_json::Value::Array(
                        allowed
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }
            // `tools` is preserved even when the list is empty: that
            // is the "MCP only, no built-ins" lockdown the rubix
            // ai-agent node forwards to `CliCfg::tools`. Collapsing
            // empty into absent would silently re-enable Bash / Read
            // / Edit and let the model curl around the MCP surface.
            if let Some(tools) = cli_tools {
                obj.insert(
                    TOOLS_KEY.to_owned(),
                    serde_json::Value::Array(
                        tools
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }
            settings = serde_json::Value::Object(obj);
        }
        let mut decl = NodeDecl::new(node_id, kind);
        decl.settings = settings;
        // Every node fires on the shared `payload` seed slot the
        // rubix surface layer writes (see
        // `boot::mcp::register::register_one`). `trigger.schedule`
        // root nodes additionally need their `cron_expr` slot
        // listed as a trigger input so the propagator copies the
        // seeded cron expression into the node's input SlotMap;
        // without this the node body errors with `trigger.schedule
        // input missing cron_expr slot`.
        let mut triggers = vec![DEFAULT_SEED_SLOT.to_owned()];
        if decl.kind.as_str() == "starter.flow.trigger.schedule" {
            triggers.push("cron_expr".to_owned());
        }
        // `starter.flow.tool-call` declares `tool_id` + `input` as
        // *read* slots intrinsically (see
        // `starter-flow-nodes/src/tool_call.rs` —
        // `NodeBehavior::read_slots`). They are available in the
        // input `SlotMap` at invoke time but writes to them do not
        // wake the node. This is the long-term event-driven
        // dataflow contract: triggers are the wake set, reads are
        // configuration / reference inputs. Re-listing them here as
        // YAML triggers would promote them back into the wake set
        // and re-introduce the per-fire multi-wake bug — so don't.
        decl.triggers = triggers;
        body.nodes.push(decl);
    }
    body.links = yaml
        .links
        .into_iter()
        .map(|l| LinkDecl::new(qualify_endpoint(&l.from), qualify_endpoint(&l.to)))
        .collect();

    // Every link's destination slot is, by construction, a slot
    // the receiving node must wake on. Add each `to` endpoint to
    // the corresponding node's `triggers` list so the propagator
    // schedules the node when an upstream link delivers a value.
    // Without this, scheduled flows like `com.rubix.tick-counter`
    // never invoke the downstream counter / log nodes \u2014 the
    // root tick node emits, the link forwards, but the propagator
    // ignores the write because the destination's `triggers` set
    // does not list the input slot.
    for link in &body.links {
        let Some((to_node, to_slot)) = link.to.rsplit_once('.') else {
            continue;
        };
        for node in body.nodes.iter_mut() {
            if node.id.as_str() == to_node && !node.triggers.iter().any(|t| t == to_slot) {
                node.triggers.push(to_slot.to_owned());
            }
        }
    }

    Ok((flow_id, FlowRevisionId::new(), body))
}

/// Rewrite a YAML link endpoint (`"<short_node>.<slot>"`) into the
/// reverse-DNS form the topology resolver expects
/// (`"<NODE_ID_PREFIX>.<short_node>.<slot>"`), matching the prefix
/// applied to every node id by [`convert`]. Endpoints that already
/// start with the prefix are passed through unchanged so flows
/// authored against the qualified form keep working.
fn qualify_endpoint(s: &str) -> String {
    if s.starts_with(&format!("{NODE_ID_PREFIX}.")) {
        s.to_owned()
    } else {
        format!("{NODE_ID_PREFIX}.{s}")
    }
}
