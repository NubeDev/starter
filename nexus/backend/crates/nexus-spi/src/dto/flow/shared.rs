//! Types shared across the flow verbs.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// A saved ingestion flow in full. The three config blobs are opaque JSON on the
/// wire — the input connector, the processor pipeline, and the output sink the
/// FlowManager hands to the engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FlowDetail {
    pub id: Uuid,
    pub name: String,
    pub input: Value,
    pub pipeline: Value,
    pub output: Value,
    /// Whether the flow is configured to run.
    pub enabled: bool,
    /// Whether the FlowManager currently has it running on this node.
    pub running: bool,
}
