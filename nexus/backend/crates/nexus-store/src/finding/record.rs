//! Row and input shapes for the findings store.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// A persistent finding — one per flagged target per detection. `target` is the
/// identifying column values; `context` is the rest of the flagged row (the
/// "why"); `status` walks the open → acknowledged → resolved lifecycle.
#[derive(Debug, Clone)]
pub struct FindingRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub detection_id: Uuid,
    pub at: DateTime<Utc>,
    pub target: Value,
    pub value: Option<f64>,
    pub context: Value,
    pub status: String,
    pub acked_by: Option<String>,
    pub acked_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
    pub dedup_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A finding that changed lifecycle state during a reconcile — the signal the
/// detection runner notifies on. `opened` is a finding that went from absent (or
/// resolved) to open this run; `resolved` is one auto-resolved because its target
/// stopped flagging. Carries just what a notification needs.
#[derive(Debug, Clone)]
pub struct FindingTransition {
    pub finding_id: Uuid,
    pub target: Value,
    pub value: Option<f64>,
    pub context: Value,
}

/// What [`super::upsert::reconcile`] changed this run: the newly opened findings
/// and the auto-resolved ones, for the runner to fan out as notifications.
#[derive(Debug, Clone, Default)]
pub struct Reconciled {
    pub opened: Vec<FindingTransition>,
    pub resolved: Vec<FindingTransition>,
}

/// A flagged row to upsert as a finding. `dedup_key` is the hash of the target
/// values: a re-flag of the same key updates the open finding instead of
/// creating a new one (see [`super::upsert::upsert`]).
#[derive(Debug, Clone)]
pub struct NewFinding {
    pub detection_id: Uuid,
    pub at: DateTime<Utc>,
    pub target: Value,
    pub value: Option<f64>,
    pub context: Value,
    pub dedup_key: String,
}

/// Filters for the findings browse query. `None` fields are unconstrained.
#[derive(Debug, Clone, Default)]
pub struct FindingFilter {
    pub detection_id: Option<Uuid>,
    pub status: Option<String>,
    /// Match findings whose `target` jsonb contains these key/value pairs
    /// (the `@>` containment operator) — e.g. `{"site":"s1"}`.
    pub target_contains: Option<Value>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: i64,
}
