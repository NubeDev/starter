//! Materialise armed alarm rules into `bc_alarms` rows.
//!
//! Only points whose `alarm_on` resolved true contribute rows, and
//! only the rules the template declared for them (BARCODE.md §5.1
//! step 5). `bc_alarms` stores the rules; *evaluation* is the host's
//! flow/anomaly-rule runtime (BARCODE.md §10 open-question 3) — this
//! extension only arms them.

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::Row;

use crate::provision::ids::alarm_id;
use crate::provision::points::ResolvedPoint;

/// Build the `bc_alarms` rows for every armed point on a device.
pub fn rows_for(device_id: &str, points: &[ResolvedPoint]) -> Vec<Row> {
    let mut rows = Vec::new();
    for p in points {
        if !p.alarm_on {
            continue;
        }
        for rule in &p.source.alarm_rules {
            let aid = alarm_id(&p.point_id, &rule.when);
            let mut row = Map::new();
            row.insert("alarm_id".into(), json!(aid));
            row.insert("device_id".into(), json!(device_id));
            row.insert("point_id".into(), json!(p.point_id));
            row.insert("point_key".into(), json!(p.point_key));
            row.insert("predicate".into(), json!(rule.when));
            row.insert("severity".into(), json!(rule.severity));
            row.insert("message".into(), json!(rule.message));
            row.insert("enabled".into(), Value::Bool(true));
            rows.push(Row::from_map(row));
        }
    }
    rows
}
