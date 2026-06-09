//! Types shared across the flow verbs.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// Live run state surfaced on the flows list and detail so an admin sees a
/// flow's health without opening it: whether it is running, when its current
/// (or most recent) run started, and the error that ended the last run (if
/// any). All fields reflect this node's in-process
/// [`FlowManager`](nexus_engine::FlowManager) state; they reset on restart and
/// are absent once the process restarts (single-node v1). Throughput counters
/// would require instrumenting every output sink, so they are intentionally
/// omitted rather than reported as zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FlowMetrics {
    /// Whether the manager currently has the flow running on this node.
    pub running: bool,
    /// RFC-3339 timestamp of when the current or most recent run started, or
    /// `None` if the flow has never run this process.
    pub last_started_at: Option<String>,
    /// The error that ended the most recent run, or `None` if it ended cleanly
    /// (or is still running).
    pub last_error: Option<String>,
}

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
    /// Live run counters for this flow.
    pub metrics: FlowMetrics,
}
