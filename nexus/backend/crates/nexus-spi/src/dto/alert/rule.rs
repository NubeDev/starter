//! Alert-rule request/response DTOs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create an alert rule: a query whose first numeric cell is compared to a
/// threshold on a cadence. `op` is one of gt|gte|lt|lte|eq|ne.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateAlertRuleRequest {
    pub name: String,
    #[serde(default)]
    pub datasource_id: Option<Uuid>,
    pub query: String,
    pub op: String,
    pub threshold: f64,
    /// Pending dwell in seconds before firing (0 = fire on first breach).
    #[serde(default)]
    pub for_secs: Option<i32>,
    /// Evaluation cadence in seconds.
    #[serde(default)]
    pub interval_secs: Option<i32>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub channel_ids: Option<Vec<Uuid>>,
}

/// Partially update an alert rule; omitted fields are unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdateAlertRuleRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default)]
    pub threshold: Option<f64>,
    #[serde(default)]
    pub for_secs: Option<i32>,
    #[serde(default)]
    pub interval_secs: Option<i32>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub channel_ids: Option<Vec<Uuid>>,
}

/// An alert rule in full.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AlertRuleDetail {
    pub id: Uuid,
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
