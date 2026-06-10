//! `PUT /api/v1/panels/:id` — update a panel.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// Partial update of a panel. `None` fields are left unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct UpdatePanelRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasource_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viz: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<Value>,
    /// Attach/replace this panel's post-query insight (RW-06). Ignored when
    /// `clear_insight` is true; `None` (with `clear_insight` false) leaves the
    /// insight unchanged. This mirrors the `folder_id` + `clear_folder` pair on
    /// dashboards — a bool flag carries the "detach" intent rather than an
    /// `Option<Option<_>>`, which serde can't distinguish from "absent" on the
    /// wire (a bug found in live testing: an explicit JSON `null` deserialized to
    /// "leave unchanged", so detach never fired).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight_id: Option<Uuid>,
    /// Detach the panel's insight (and clear its params). Takes precedence over
    /// `insight_id`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clear_insight: bool,
    /// Set the insight's params. Ignored when `clear_insight` is true; cleared
    /// alongside the insight on detach. `None` leaves params unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insight_params: Option<Value>,
}
