//! Detection DTOs — the saved analytic rules that run on a schedule and emit
//! findings (WS-15). A detection references a stored insight (the Rhai rule) and
//! adds the query, the schedule, and the column mapping that turns flagged rows
//! into findings.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// Create a detection. `insight_id` is the stored Rhai rule (required — a
/// detection with no rule is meaningless); `flag_column` names the insight
/// output column whose truthy value marks a flagged row; `target_columns`
/// identify the finding's target and form its dedup key; `value_column` is the
/// numeric column carried onto the finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateDetectionRequest {
    pub name: String,
    pub insight_id: Uuid,
    #[serde(default)]
    pub datasource_id: Option<Uuid>,
    pub sql: String,
    /// Params passed to the insight (thresholds, window, z, …).
    #[serde(default)]
    pub params: Option<Value>,
    /// RW-05 federated sources the `sql` joins over (alias → datasource ref).
    /// Empty (the default) runs the single-datasource push-down path; non-empty
    /// dispatches through the federation engine, exactly like a panel query.
    #[serde(default)]
    pub sources: Option<Value>,
    pub flag_column: String,
    #[serde(default)]
    pub target_columns: Vec<String>,
    #[serde(default)]
    pub value_column: Option<String>,
    /// Optional dwell in seconds before a flagged row becomes a finding
    /// (default 0 — most analytic findings are point-in-time).
    #[serde(default)]
    pub for_secs: Option<i32>,
    /// Run cadence in seconds (default 300).
    #[serde(default)]
    pub interval_secs: Option<i32>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// Partially update a detection; omitted fields are unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdateDetectionRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub insight_id: Option<Uuid>,
    /// Point the query at this datasource. `None` (with `clear_datasource` false)
    /// leaves it unchanged. A `clear_datasource` bool carries the "detach → dev
    /// pool" intent rather than an `Option<Option<_>>` — serde can't distinguish
    /// an explicit JSON `null` from "absent" on the wire (the same bug the panel
    /// `clear_insight` flag avoids; confirmed live for detections too).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<Uuid>,
    /// Clear the datasource (run against the dev pool). Takes precedence over
    /// `datasource_id`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clear_datasource: bool,
    #[serde(default)]
    pub sql: Option<String>,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub sources: Option<Value>,
    #[serde(default)]
    pub flag_column: Option<String>,
    #[serde(default)]
    pub target_columns: Option<Vec<String>>,
    #[serde(default)]
    pub value_column: Option<String>,
    #[serde(default)]
    pub for_secs: Option<i32>,
    #[serde(default)]
    pub interval_secs: Option<i32>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// A detection as returned by the API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DetectionDetail {
    pub id: Uuid,
    pub name: String,
    pub insight_id: Uuid,
    pub datasource_id: Option<Uuid>,
    pub sql: String,
    pub params: Value,
    pub sources: Value,
    pub flag_column: String,
    pub target_columns: Vec<String>,
    pub value_column: Option<String>,
    pub for_secs: i32,
    pub interval_secs: i32,
    pub enabled: bool,
}

/// One persistent finding emitted by a detection run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Finding {
    pub id: Uuid,
    pub detection_id: Uuid,
    pub at: DateTime<Utc>,
    /// The identifying column values, e.g. `{"site":"s1","meter":"m7"}`.
    pub target: Value,
    pub value: Option<f64>,
    /// The flagged row's other derived columns — the "why".
    pub context: Value,
    /// open | acknowledged | resolved.
    pub status: String,
    pub acked_by: Option<String>,
    pub acked_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

/// Run stats for a detection — a glanceable summary for the list/editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DetectionStats {
    /// When the scheduler will next run this detection.
    pub next_eval_at: DateTime<Utc>,
    /// The most recent finding time across all this detection's findings.
    pub last_finding_at: Option<DateTime<Utc>>,
    pub open: i64,
    pub acknowledged: i64,
    pub resolved: i64,
    pub total: i64,
}

/// Acknowledge or manually resolve a finding; both accept an optional note.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FindingActionRequest {
    #[serde(default)]
    pub note: Option<String>,
}
