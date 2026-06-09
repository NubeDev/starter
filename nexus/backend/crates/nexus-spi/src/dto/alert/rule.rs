//! Alert-rule request/response DTOs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::condition::AlertCondition;

/// Create an alert rule. A rule is either a legacy single condition (the
/// top-level `query`/`op`/`threshold`) or a multi-condition rule (`conditions`
/// combined by `combinator`). `no_data_policy`/`exec_error_policy` say how a
/// missing or failed evaluation resolves (`ok`|`alerting`|`keep_last`).
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
    /// Multi-condition list; when set it supersedes the single `query`/`op`.
    #[serde(default)]
    pub conditions: Option<Vec<AlertCondition>>,
    /// How conditions combine: `and`|`or` (default `and`).
    #[serde(default)]
    pub combinator: Option<String>,
    /// No-data policy: `ok`|`alerting`|`keep_last` (default `ok`).
    #[serde(default)]
    pub no_data_policy: Option<String>,
    /// Execution-error policy: `ok`|`alerting`|`keep_last` (default `ok`).
    #[serde(default)]
    pub exec_error_policy: Option<String>,
    /// Optional notification message template; omitted uses the default.
    #[serde(default)]
    pub message_template: Option<String>,
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
    #[serde(default)]
    pub conditions: Option<Vec<AlertCondition>>,
    #[serde(default)]
    pub combinator: Option<String>,
    #[serde(default)]
    pub no_data_policy: Option<String>,
    #[serde(default)]
    pub exec_error_policy: Option<String>,
    #[serde(default)]
    pub message_template: Option<String>,
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
    /// The multi-condition list, when the rule uses one. `None` for a legacy
    /// single-condition rule (its single condition is the top-level fields).
    pub conditions: Option<Vec<AlertCondition>>,
    pub combinator: String,
    pub no_data_policy: String,
    pub exec_error_policy: String,
    pub message_template: Option<String>,
}
