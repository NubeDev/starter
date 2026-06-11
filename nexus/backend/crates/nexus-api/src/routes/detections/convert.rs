//! Store-record ⇄ DTO mapping for detections and findings.

use nexus_spi::dto::detection::{DetectionDetail, DetectionStats, Finding};
use nexus_store::detection::{DetectionRecord, DetectionStats as StoreStats};
use nexus_store::finding::FindingRecord;

pub fn detection_to_detail(r: &DetectionRecord) -> DetectionDetail {
    DetectionDetail {
        id: r.id,
        name: r.name.clone(),
        insight_id: r.insight_id,
        datasource_id: r.datasource_id,
        sql: r.sql.clone(),
        params: r.params.clone(),
        sources: r.sources.clone(),
        flag_column: r.flag_column.clone(),
        target_columns: r.target_columns.clone(),
        value_column: r.value_column.clone(),
        for_secs: r.for_secs,
        interval_secs: r.interval_secs,
        enabled: r.enabled,
    }
}

pub fn stats_to_dto(s: &StoreStats) -> DetectionStats {
    DetectionStats {
        next_eval_at: s.next_eval_at,
        last_finding_at: s.last_finding_at,
        open: s.open,
        acknowledged: s.acknowledged,
        resolved: s.resolved,
        total: s.total,
    }
}

pub fn finding_to_dto(r: &FindingRecord) -> Finding {
    Finding {
        id: r.id,
        detection_id: r.detection_id,
        at: r.at,
        target: r.target.clone(),
        value: r.value,
        context: r.context.clone(),
        status: r.status.clone(),
        acked_by: r.acked_by.clone(),
        acked_at: r.acked_at,
        resolved_at: r.resolved_at,
        note: r.note.clone(),
    }
}
