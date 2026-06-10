//! `GET /api/v1/flows/node-types` response — the flow-builder palette.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

/// Which palette group a node belongs to. The visual builder groups the palette
/// by this and constrains edges (input → processor* → output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    /// A flow source.
    Input,
    /// A pipeline transform.
    Processor,
    /// A flow sink.
    Output,
}

/// A node type the engine can build, described for the editor: its engine
/// `type` discriminant, palette grouping, labels, and a JSON Schema for its
/// config so the editor can render a schema-driven form instead of raw JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NodeType {
    /// The engine `type` value the serialised graph node carries.
    pub kind: String,
    /// Palette group.
    pub category: NodeCategory,
    /// Human label for the palette.
    pub label: String,
    /// One-line description.
    pub description: String,
    /// JSON Schema (draft 2020-12) for the node's config object.
    pub config_schema: Value,
}

/// The full palette: every registered node, in palette order (inputs,
/// processors, outputs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NodeTypeList {
    pub node_types: Vec<NodeType>,
}
