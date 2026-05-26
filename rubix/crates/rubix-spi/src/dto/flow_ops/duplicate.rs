//! `rubix.flow_ops.duplicate` — request/response DTOs and tool descriptor.
//!
//! Reads the latest live revision of `source_flow_id` and writes a
//! fresh revision under `target_flow_id`. The body YAML's `id:`
//! field is rewritten to match `target_flow_id` so the duplicate
//! is loadable straight away. See
//! [docs/design/flows/](../../../../docs/design/flows/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.flow_ops.duplicate`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowDuplicateRequest {
    /// Existing flow to clone the latest revision of.
    pub source_flow_id: String,
    /// New flow id for the duplicate. Must not already have a live
    /// revision; the verb refuses to overwrite.
    pub target_flow_id: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowDuplicateResponse {
    /// Outcome (`rubix.flow.duplicated`).
    pub summary: Diagnostic,
    /// Echoed source.
    pub source_flow_id: String,
    /// Echoed target.
    pub target_flow_id: String,
    /// Stable id of the new revision row.
    pub revision_id: String,
    /// Epoch milliseconds (UTC) at which the row was created.
    pub created_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "flows.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Duplicate a rubix flow under a new id by copying its latest live revision.",
    when_to_use: concat!(
        "Use when an operator says \"fork flow X to Y\" or when a flow ",
        "wants a starting point to edit without disturbing the original."
    ),
    when_not_to_use: concat!(
        "Do not use to deploy a hand-edited body — that is ",
        "rubix.flow_ops.deploy. Do not use when the target id already ",
        "has a live revision; this verb refuses to overwrite."
    ),
    example: concat!(
        "Input:  { \"source_flow_id\": \"com.rubix.flow-programmer\", ",
        "\"target_flow_id\": \"com.example.flow-programmer-copy\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.flow.duplicated\", ",
        "\"params\": { \"source\": \"com.rubix.flow-programmer\", ",
        "\"target\": \"com.example.flow-programmer-copy\" } }, ",
        "\"source_flow_id\": \"com.rubix.flow-programmer\", ",
        "\"target_flow_id\": \"com.example.flow-programmer-copy\", ",
        "\"revision_id\": \"...\", \"created_at_ms\": 1764892800000 }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.flow_ops.deploy",
        wins_when: "the caller already has the desired YAML body in hand.",
    }],
};
