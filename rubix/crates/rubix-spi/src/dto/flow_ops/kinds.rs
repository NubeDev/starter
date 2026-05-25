//! `rubix.flow_ops.kinds` — request/response DTOs and tool descriptor.
//!
//! Read-only verb: enumerates every node kind the agent's
//! [`starter_flow::registry::NodeKindRegistry`] currently knows about,
//! returning the reverse-DNS kind id, the kind's `config_schema` (a
//! JSON Schema document), and a `default_label` suitable for
//! catalog-row rendering when the i18n catalog has not yet resolved.
//! SELECT-free: the response is built from in-process state.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.flow_ops.kinds`. Empty — listing is
/// unfiltered (the registry's full surface is cheap to enumerate).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct FlowKindsRequest {}

/// One registered node kind as returned by `rubix.flow_ops.kinds`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowKindItem {
    /// Reverse-DNS kind id (matches [`starter_flow_spi::node::KindId`]).
    pub kind_id: String,
    /// JSON Schema describing the kind's `settings:` shape — the
    /// schema returned by `NodeBehavior::config_schema()`.
    #[schema(value_type = Object)]
    pub config_schema: Value,
    /// Human-readable label used when the i18n catalog is not loaded
    /// (typically the last reverse-DNS segment title-cased).
    pub default_label: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowKindsResponse {
    /// Outcome (`rubix.flow.kinds.listed`).
    pub summary: Diagnostic,
    /// Number of kinds returned.
    pub count: usize,
    /// Kinds sorted by `kind_id` ascending for stable rendering.
    pub kinds: Vec<FlowKindItem>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "flows.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List every node kind the rubix flow runtime knows about.",
    when_to_use: concat!(
        "Use to populate a flow-designer palette or settings sidebar — the ",
        "response is everything a UI needs to render a 'kinds' catalog row ",
        "(reverse-DNS id, default label, JSON Schema for the settings form)."
    ),
    when_not_to_use: concat!(
        "Do not use to look up a specific kind's behaviour or run a node — ",
        "this verb is descriptive only."
    ),
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.flow.kinds.listed\" }, ",
        "\"count\": 2, \"kinds\": [ ",
        "{ \"kind_id\": \"starter.flow.counter\", \"default_label\": \"Counter\", ",
        "\"config_schema\": { \"type\": \"object\", ... } }, ... ] }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.flow_ops.list",
        wins_when: "the caller wants the LIVE FLOW ROWS (not the available kinds catalog).",
    }],
};
