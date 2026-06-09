//! Silence (maintenance-window) request/response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create a silence. `rule_id = None` silences every rule in the tenant for the
/// window; a value silences just that rule. Silencing suppresses notification,
/// not evaluation — the event history still records what fired.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateSilenceRequest {
    #[serde(default)]
    pub rule_id: Option<Uuid>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// A silence window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SilenceDetail {
    pub id: Uuid,
    pub rule_id: Option<Uuid>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub reason: Option<String>,
}
