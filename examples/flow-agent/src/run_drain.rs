//! Bridge-side run draining + slot → JSON conversion.
//!
//! Split out of [`crate::agent_bridge`] for the workspace 400-line
//! rule. Owns the small adapter that takes a [`FireOutcome`] (the
//! handle the flow engine returns from `FlowEngine::fire`) and pumps
//! its `FlowEvent`s onto the shared `EventHub.runs` channel until the
//! run terminates — so agent-fired runs and user-fired runs surface
//! the same overlay signal to the editor.

use std::sync::Arc;

use serde_json::{json, Value as JsonValue};

use crate::flow_engine::FireOutcome;
use crate::sse::{EventHub, RunEvent};

/// Pump engine `FlowEvent`s onto `EventHub.runs` until the run
/// terminates. Returns `(status, trace_json, output_map_json)`. The
/// output map is the engine's `RunCompleted.output` SlotMap converted
/// to JSON for inclusion in the bridge's tool-result frame.
pub(crate) async fn drain_run(
    hub: Arc<EventHub>,
    flow_id: String,
    run_db_id: String,
    mut outcome: FireOutcome,
) -> (String, Option<JsonValue>, JsonValue) {
    use starter_flow_spi::flow::FlowEvent as EngineFlowEvent;
    use tokio::sync::broadcast::error::RecvError;

    let mut terminal_status = "error".to_owned();
    let mut terminal_trace: Option<JsonValue> = None;
    let mut terminal_output: JsonValue = json!({});

    let mut rx = std::mem::replace(
        &mut outcome.handle.initial_rx,
        outcome.handle.events_tx.subscribe(),
    );

    loop {
        match rx.recv().await {
            Ok(ev) => match ev {
                EngineFlowEvent::NodeStarted { node, .. } => {
                    if let Some(ui_id) = outcome.ui_node_id(&node) {
                        let _ = hub.runs.send(RunEvent::NodeStatus {
                            flow_id: flow_id.clone(),
                            run_id: run_db_id.clone(),
                            node_id: ui_id.to_owned(),
                            status: "running".into(),
                        });
                    }
                }
                EngineFlowEvent::NodeEmitted { node, slot, .. } => {
                    if let Some(edges) = outcome.edge_index.get(&(node.clone(), slot.clone())) {
                        for edge_id in edges {
                            let _ = hub.runs.send(RunEvent::EdgeActive {
                                flow_id: flow_id.clone(),
                                run_id: run_db_id.clone(),
                                edge_id: edge_id.clone(),
                            });
                        }
                    }
                    if let Some(ui_id) = outcome.ui_node_id(&node) {
                        let _ = hub.runs.send(RunEvent::NodeStatus {
                            flow_id: flow_id.clone(),
                            run_id: run_db_id.clone(),
                            node_id: ui_id.to_owned(),
                            status: "ok".into(),
                        });
                    }
                }
                EngineFlowEvent::NodeFailed { node, error, .. } => {
                    if let Some(ui_id) = outcome.ui_node_id(&node) {
                        let _ = hub.runs.send(RunEvent::NodeStatus {
                            flow_id: flow_id.clone(),
                            run_id: run_db_id.clone(),
                            node_id: ui_id.to_owned(),
                            status: "error".into(),
                        });
                    }
                    tracing::warn!(node = %node, error = %error, "flow node failed (bridge)");
                }
                EngineFlowEvent::RunCompleted { output, .. } => {
                    terminal_status = "ok".into();
                    terminal_output = slotmap_to_json(&output);
                    break;
                }
                EngineFlowEvent::RunFailed { error, .. } => {
                    terminal_status = "error".into();
                    terminal_trace = Some(json!({ "error": error }));
                    break;
                }
                EngineFlowEvent::RunCancelled { .. } => {
                    terminal_status = "cancelled".into();
                    break;
                }
                _ => {}
            },
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }
    let _ = outcome.handle.join.await;
    (terminal_status, terminal_trace, terminal_output)
}

fn slotmap_to_json(map: &starter_flow_spi::node::SlotMap) -> JsonValue {
    let mut obj = serde_json::Map::new();
    for (k, v) in map.iter() {
        obj.insert(k.clone(), slotvalue_to_json(v));
    }
    JsonValue::Object(obj)
}

fn slotvalue_to_json(v: &starter_flow_spi::node::SlotValue) -> JsonValue {
    use starter_flow_spi::node::SlotValue;
    match v {
        SlotValue::Null => JsonValue::Null,
        SlotValue::String(s) => JsonValue::String(s.clone()),
        SlotValue::Int(i) => JsonValue::from(*i),
        SlotValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        SlotValue::Bool(b) => JsonValue::Bool(*b),
        SlotValue::Json(j) => j.clone(),
        SlotValue::Bytes(b) => JsonValue::String(format!("<{} bytes>", b.len())),
        _ => JsonValue::Null,
    }
}
