//! Row and input shapes for the notification store.

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// A notification delivery target. `kind` selects the notifier impl
/// (webhook|slack|email); `config` is its kind-specific settings (jsonb), with
/// secrets redacted on the read path.
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

/// A maintenance-window silence: suppresses delivery (not detection runs) for
/// one detection, or — when `detection_id` is None — for the whole tenant.
#[derive(Debug, Clone)]
pub struct SilenceRecord {
    pub id: Uuid,
    pub detection_id: Option<Uuid>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub reason: Option<String>,
}

/// A new silence to insert.
#[derive(Debug, Clone)]
pub struct NewSilence {
    pub detection_id: Option<Uuid>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub reason: Option<String>,
    /// Subject of the principal who created the silence, for the audit trail.
    pub created_by: String,
}

/// An append-only notification event: one per finding transition the runner
/// tried to notify on.
#[derive(Debug, Clone)]
pub struct NotifyEventRecord {
    pub id: Uuid,
    pub detection_id: Uuid,
    pub finding_id: Option<Uuid>,
    pub at: DateTime<Utc>,
    /// The finding transition: opened | resolved.
    pub transition: String,
    pub value: Option<f64>,
    pub silenced: bool,
    pub notified: bool,
    pub detail: Option<String>,
}

/// A new notify event to append.
#[derive(Debug, Clone)]
pub struct NewNotifyEvent {
    pub detection_id: Uuid,
    pub finding_id: Option<Uuid>,
    pub transition: String,
    pub value: Option<f64>,
    pub silenced: bool,
    pub notified: bool,
    pub detail: Option<String>,
}
