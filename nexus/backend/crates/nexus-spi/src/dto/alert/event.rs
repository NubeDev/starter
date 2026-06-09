//! Alert-event (transition history) response DTO.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// One recorded transition: when a rule fired or resolved, the value that
/// triggered it, and whether it was silenced / actually notified.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AlertEvent {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub at: DateTime<Utc>,
    /// firing | resolved.
    pub transition: String,
    pub value: Option<f64>,
    pub silenced: bool,
    pub notified: bool,
    pub detail: Option<String>,
}
