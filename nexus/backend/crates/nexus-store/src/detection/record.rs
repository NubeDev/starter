//! Row and input shapes for the detection store.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// A saved detection: a stored insight + a query + a schedule + the column
/// mapping that turns flagged rows into findings. `flag_column` is the insight
/// output column whose truthy value means "flagged"; `target_columns` identify
/// the finding's target (and form its dedup key); `value_column` is carried onto
/// the finding as its numeric value.
#[derive(Debug, Clone)]
pub struct DetectionRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub insight_id: Uuid,
    pub datasource_id: Option<Uuid>,
    pub sql: String,
    pub params: Value,
    /// RW-05 federated sources (alias → datasource ref) the `sql` joins over. An
    /// empty array keeps the detection on the single-datasource push-down path;
    /// a non-empty one dispatches through the federation engine (cross-datasource
    /// or file joins), matching a panel query exactly.
    pub sources: Value,
    pub flag_column: String,
    pub target_columns: Vec<String>,
    pub value_column: Option<String>,
    pub for_secs: i32,
    pub interval_secs: i32,
    pub enabled: bool,
}

/// Run stats for one detection — a glanceable summary of what it's producing.
/// `next_eval_at` is when the scheduler will next run it; the counts are the
/// detection's findings by status; `last_finding_at` is the most recent `at`
/// across them (None until the first run flags something).
#[derive(Debug, Clone)]
pub struct DetectionStats {
    pub next_eval_at: DateTime<Utc>,
    pub last_finding_at: Option<DateTime<Utc>>,
    pub open: i64,
    pub acknowledged: i64,
    pub resolved: i64,
    pub total: i64,
}

/// A new detection to insert.
#[derive(Debug, Clone)]
pub struct NewDetection {
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

/// A partial detection update; `None` fields are unchanged.
#[derive(Debug, Clone, Default)]
pub struct DetectionPatch {
    pub name: Option<String>,
    pub insight_id: Option<Uuid>,
    /// Double-option: `None` leaves the datasource unchanged; `Some(None)` clears
    /// it (dev pool); `Some(Some(id))` repoints it. Mirrors the DTO so the route
    /// threads the three-state choice straight through.
    pub datasource_id: Option<Option<Uuid>>,
    pub sql: Option<String>,
    pub params: Option<Value>,
    pub sources: Option<Value>,
    pub flag_column: Option<String>,
    pub target_columns: Option<Vec<String>>,
    pub value_column: Option<String>,
    pub for_secs: Option<i32>,
    pub interval_secs: Option<i32>,
    pub enabled: Option<bool>,
}
