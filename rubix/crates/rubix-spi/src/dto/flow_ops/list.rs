//! `rubix.flow_ops.list` — request/response DTOs and tool descriptor.
//!
//! Read-only verb: returns the set of distinct `flow_id`s with a
//! live (non-superseded) revision in `flows_definitions`. See
//! [docs/design/flows/](../../../../docs/design/flows/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.flow_ops.list`.
///
/// Empty for v1 — listing is unfiltered. Future revisions will add
/// optional `tenant_id` / `prefix` filters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct FlowListRequest {}

/// One live flow as returned by `rubix.flow_ops.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowListItem {
    /// Reverse-DNS flow id.
    pub flow_id: String,
    /// Revision id of the currently-live row for this flow.
    pub revision_id: String,
    /// Raw YAML body of the live revision. Returned inline from the
    /// same row the live-revision SELECT already fetches — no extra
    /// round-trip — so the frontend can render a flow's full graph
    /// from a single `flow_ops.list` response without needing a
    /// follow-up `flow_ops.get` verb.
    pub body_yaml: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowListResponse {
    /// Outcome (`rubix.flow.listed`).
    pub summary: Diagnostic,
    /// Total row count surfaced.
    pub count: usize,
    /// Rows sorted by `flow_id` ascending for stable rendering.
    pub flows: Vec<FlowListItem>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "flows.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List rubix flows that have a live revision.",
    when_to_use: concat!(
        "Use to inspect which flows the agent currently serves, or as ",
        "a quick post-deploy sanity check (the new flow_id should be ",
        "in the list)."
    ),
    when_not_to_use: concat!(
        "Do not use to fetch a specific flow's body — that is the role ",
        "of a future rubix.flow_ops.get verb."
    ),
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.flow.listed\", ",
        "\"params\": { \"count\": 6 } }, \"count\": 6, ",
        "\"flows\": [ { \"flow_id\": \"com.rubix.flow-programmer\", ",
        "\"revision_id\": \"...\" }, ... ] }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.flow_ops.duplicate",
        wins_when: "the caller wants to FORK one of the listed flows under a new id.",
    }],
};
