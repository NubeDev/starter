//! Expand a template's points into `bc_points` rows for one device.
//!
//! Each [`ExpandedPoint`] becomes one row keyed by `device_id:key`.
//! `trend_on` / `alarm_on` resolve as `master_toggle ?? template
//! default`: an explicit master toggle on the provision request wins;
//! otherwise the per-point template default applies (BARCODE.md §5.1
//! step 4).

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::Row;

use crate::provision::ids::point_id;
use crate::provision::template::ExpandedPoint;

/// Master toggles from the provision request. `None` means "fall back
/// to the per-point template default".
#[derive(Debug, Clone, Copy, Default)]
pub struct Toggles {
    pub trend: Option<bool>,
    pub alarm: Option<bool>,
}

impl Toggles {
    fn trend_for(&self, p: &ExpandedPoint) -> bool {
        self.trend.unwrap_or(p.trend_default)
    }

    /// A point can only arm an alarm if the template gave it rules.
    fn alarm_for(&self, p: &ExpandedPoint) -> bool {
        if p.alarm_rules.is_empty() {
            return false;
        }
        self.alarm.unwrap_or(p.alarm_default)
    }
}

/// One resolved point: the row to persist plus the toggle decisions
/// downstream stages (alarms, widgets) need.
pub struct ResolvedPoint {
    pub point_id: String,
    pub point_key: String,
    pub widget: Option<String>,
    pub alarm_on: bool,
    pub source: ExpandedPoint,
    row: Map<String, Value>,
}

impl ResolvedPoint {
    /// The `bc_points` row to insert.
    pub fn row(&self) -> Row {
        Row::from_map(self.row.clone())
    }
}

/// Resolve every expanded point against the master toggles.
pub fn resolve(
    device_id: &str,
    points: &[ExpandedPoint],
    toggles: Toggles,
) -> Vec<ResolvedPoint> {
    points
        .iter()
        .map(|p| {
            let pid = point_id(device_id, &p.key);
            let trend_on = toggles.trend_for(p);
            let alarm_on = toggles.alarm_for(p);
            let mut row = Map::new();
            row.insert("point_id".into(), json!(pid));
            row.insert("device_id".into(), json!(device_id));
            row.insert("point_key".into(), json!(p.key));
            row.insert("name".into(), json!(p.name));
            row.insert("unit".into(), json!(p.unit));
            row.insert("kind".into(), json!(p.kind));
            row.insert("widget".into(), json!(p.widget));
            row.insert("writable".into(), json!(p.writable));
            row.insert("trend_on".into(), json!(trend_on));
            row.insert("alarm_on".into(), json!(alarm_on));
            row.insert("trend_interval".into(), json!(p.trend_interval));
            ResolvedPoint {
                point_id: pid.clone(),
                point_key: p.key.clone(),
                widget: p.widget.clone(),
                alarm_on,
                source: p.clone(),
                row,
            }
        })
        .collect()
}
