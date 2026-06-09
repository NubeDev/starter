//! Map store flow records to wire DTOs, folding in the manager's run state.

use nexus_engine::FlowManager;
use nexus_spi::dto::flow::{FlowDetail, FlowSummary};
use nexus_store::flow::FlowRecord;

/// Full detail, with `running` reflecting whether the manager has it live now.
pub fn to_detail(rec: &FlowRecord, flows: &FlowManager) -> FlowDetail {
    FlowDetail {
        id: rec.id,
        name: rec.name.clone(),
        input: rec.input.clone(),
        pipeline: rec.pipeline.clone(),
        output: rec.output.clone(),
        enabled: rec.enabled,
        running: flows.is_running(&rec.id.to_string()),
    }
}

/// List item, likewise carrying live run state.
pub fn to_summary(rec: &FlowRecord, flows: &FlowManager) -> FlowSummary {
    FlowSummary {
        id: rec.id,
        name: rec.name.clone(),
        enabled: rec.enabled,
        running: flows.is_running(&rec.id.to_string()),
    }
}
