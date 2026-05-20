//! SSE broadcast hubs.
//!
//! Two channels:
//! - `sidebar` — `FlowEvent` (flow-created / -renamed / -deleted) for
//!   the nested sidebar tree.
//! - `runs`    — per-flow `RunEvent` (node-status / edge-active /
//!   run-started / run-finished). Filtered server-side by `flow_id`.
//!
//! Phase 1 only wires the sidebar hub end-to-end; the run-event hub
//! is plumbed in Phase 3 once `starter-flow` is wired.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use utoipa::ToSchema;

const SIDEBAR_CAP: usize = 128;
const RUN_CAP: usize = 256;

#[derive(Clone)]
pub struct EventHub {
    pub sidebar: broadcast::Sender<FlowEvent>,
    pub runs: broadcast::Sender<RunEvent>,
}

impl EventHub {
    pub fn new() -> Self {
        let (sidebar, _) = broadcast::channel(SIDEBAR_CAP);
        let (runs, _) = broadcast::channel(RUN_CAP);
        Self { sidebar, runs }
    }
}

impl Default for EventHub {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum FlowEvent {
    FlowCreated {
        id: String,
        name: String,
    },
    FlowRenamed {
        id: String,
        name: String,
    },
    FlowDeleted {
        id: String,
    },
    AgentCreated {
        id: String,
        name: String,
    },
    AgentRenamed {
        id: String,
        name: String,
    },
    AgentDeleted {
        id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RunEvent {
    RunStarted {
        flow_id: String,
        run_id: String,
    },
    NodeStatus {
        flow_id: String,
        run_id: String,
        node_id: String,
        status: String,
    },
    EdgeActive {
        flow_id: String,
        run_id: String,
        edge_id: String,
    },
    /// Value written to an output slot. Echoes `FlowEvent::NodeEmitted`
    /// from the engine, with the engine node id translated back to
    /// the UI id and the value serialized as JSON.
    NodeOutput {
        flow_id: String,
        run_id: String,
        node_id: String,
        slot: String,
        value: serde_json::Value,
    },
    RunFinished {
        flow_id: String,
        run_id: String,
        status: String,
    },
}
