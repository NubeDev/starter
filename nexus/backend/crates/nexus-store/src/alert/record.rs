//! Row and input shapes for the alerting store.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// A saved alert rule: a query compared to a threshold on a cadence.
#[derive(Debug, Clone)]
pub struct RuleRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub datasource_id: Option<Uuid>,
    pub query: String,
    pub op: String,
    pub threshold: f64,
    pub for_secs: i32,
    pub interval_secs: i32,
    pub enabled: bool,
    pub channel_ids: Vec<Uuid>,
}

/// A new rule to insert.
#[derive(Debug, Clone)]
pub struct NewRule {
    pub name: String,
    pub datasource_id: Option<Uuid>,
    pub query: String,
    pub op: String,
    pub threshold: f64,
    pub for_secs: i32,
    pub interval_secs: i32,
    pub enabled: bool,
    pub channel_ids: Vec<Uuid>,
}

/// A partial rule update; `None` fields are unchanged.
#[derive(Debug, Clone, Default)]
pub struct RulePatch {
    pub name: Option<String>,
    pub query: Option<String>,
    pub op: Option<String>,
    pub threshold: Option<f64>,
    pub for_secs: Option<i32>,
    pub interval_secs: Option<i32>,
    pub enabled: Option<bool>,
    pub channel_ids: Option<Vec<Uuid>>,
}

/// The persisted state-machine memory for one rule.
#[derive(Debug, Clone)]
pub struct RuleState {
    pub rule_id: Uuid,
    pub state: String,
    pub since: DateTime<Utc>,
    pub last_eval_at: Option<DateTime<Utc>>,
    pub last_value: Option<f64>,
}

/// An append-only transition record.
#[derive(Debug, Clone)]
pub struct EventRecord {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub at: DateTime<Utc>,
    pub transition: String,
    pub value: Option<f64>,
    pub silenced: bool,
    pub notified: bool,
    pub detail: Option<String>,
}

/// A new transition to append.
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub rule_id: Uuid,
    pub transition: String,
    pub value: Option<f64>,
    pub silenced: bool,
    pub notified: bool,
    pub detail: Option<String>,
}

/// A notification target.
#[derive(Debug, Clone)]
pub struct ChannelRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub kind: String,
    pub config: Value,
}

/// A new channel to insert.
#[derive(Debug, Clone)]
pub struct NewChannel {
    pub name: String,
    pub kind: String,
    pub config: Value,
}

/// A maintenance-window silence.
#[derive(Debug, Clone)]
pub struct SilenceRecord {
    pub id: Uuid,
    pub rule_id: Option<Uuid>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub reason: Option<String>,
}

/// A new silence to insert.
#[derive(Debug, Clone)]
pub struct NewSilence {
    pub rule_id: Option<Uuid>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub reason: Option<String>,
    /// Subject of the principal who created the silence, for the audit trail.
    pub created_by: String,
}
