//! `rubix.flow_ops.deploy` — request/response DTOs and tool descriptor.
//!
//! Deploys a new revision of a flow definition: the YAML body is
//! validated through `rubix_flows::yaml::RubixFlowYaml`, persisted
//! as a new row in the `flows_definitions` dimension table, and the
//! previously-live revision for the same `flow_id` is marked
//! `superseded_at = now()`. See
//! [docs/design/flows/](../../../../docs/design/flows/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.flow_ops.deploy`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowDeployRequest {
    /// Reverse-DNS flow id. Must match the `id:` field inside
    /// `body_yaml` — the verb cross-checks the two and refuses a
    /// mismatched request.
    pub flow_id: String,
    /// Raw YAML body, persisted verbatim.
    pub body_yaml: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowDeployResponse {
    /// Outcome (`rubix.flow.deployed`).
    pub summary: Diagnostic,
    /// Echoed flow id.
    pub flow_id: String,
    /// Stable id of the new revision row (UUID rendered as text).
    pub revision_id: String,
    /// Revision id of the row this deploy superseded, or `None`
    /// when this was the first revision for `flow_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_revision_id: Option<String>,
    /// Epoch milliseconds (UTC) at which the revision landed.
    pub deployed_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "flows.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Deploy a new revision of a rubix flow from a YAML body.",
    when_to_use: concat!(
        "Use when an operator says \"deploy this flow\", \"publish a ",
        "new version of flow X\", or when an editor finishes a draft ",
        "and wants the agent loop to pick it up on the next NOTIFY."
    ),
    when_not_to_use: concat!(
        "Do not use to roll back to a prior revision (call ",
        "rubix.undo.last against the deploy). Do not use to validate ",
        "without writing — that is rubix.flow_ops.lint."
    ),
    example: concat!(
        "Input:  { \"flow_id\": \"com.rubix.flow-programmer\", ",
        "\"body_yaml\": \"id: com.rubix.flow-programmer\\n...\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.flow.deployed\", ",
        "\"params\": { \"flow_id\": \"com.rubix.flow-programmer\" } }, ",
        "\"flow_id\": \"com.rubix.flow-programmer\", ",
        "\"revision_id\": \"...\", \"prior_revision_id\": \"...\", ",
        "\"deployed_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.flow_ops.lint",
            wins_when: "the caller wants to validate WITHOUT writing a revision.",
        },
        SiblingTool {
            id: "rubix.flow_ops.duplicate",
            wins_when: "the caller wants a copy of an existing flow under a new id rather than editing one in place.",
        },
    ],
};
