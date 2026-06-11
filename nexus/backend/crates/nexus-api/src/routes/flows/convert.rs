//! Map store flow records to wire DTOs, folding in the manager's run state.

use nexus_engine::FlowManager;
use nexus_spi::dto::flow::{FlowDetail, FlowMetrics, FlowSummary};
use nexus_store::flow::FlowRecord;

/// Build the live-run metrics for `id` from the manager's running set and the
/// last-run stats it retains.
fn metrics_of(id: &str, flows: &FlowManager) -> FlowMetrics {
    let stats = flows.stats(id);
    let m = stats.metrics;
    FlowMetrics {
        running: flows.is_running(id),
        last_started_at: stats.last_started_at,
        last_error: stats.last_error,
        batches_in: m.batches_in,
        rows_written: m.rows_written,
        channel_depth: m.channel_depth,
        flush_count: m.flush_count,
        write_errors: m.write_errors,
        last_write_ms: m.last_write_ms,
    }
}

/// Full detail, with `running`/`metrics` reflecting the manager's live state.
pub fn to_detail(rec: &FlowRecord, flows: &FlowManager) -> FlowDetail {
    let id = rec.id.to_string();
    FlowDetail {
        id: rec.id,
        name: rec.name.clone(),
        input: rec.input.clone(),
        pipeline: rec.pipeline.clone(),
        output: rec.output.clone(),
        enabled: rec.enabled,
        running: flows.is_running(&id),
        metrics: metrics_of(&id, flows),
    }
}

/// List item, likewise carrying live run state.
pub fn to_summary(rec: &FlowRecord, flows: &FlowManager) -> FlowSummary {
    let id = rec.id.to_string();
    FlowSummary {
        id: rec.id,
        name: rec.name.clone(),
        enabled: rec.enabled,
        running: flows.is_running(&id),
        metrics: metrics_of(&id, flows),
    }
}
