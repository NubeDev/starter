//! Update-nav-node request (WS-13 §4). Partial: omitted fields are unchanged.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::shared::{NavContext, NavTarget};

/// Partially update a nav node. Omitted fields are left unchanged. The
/// three-valued fields (parent, context, icon, accent) pair an optional value
/// with a `clear_*` flag so JSON can express "leave / set / clear" without a
/// nested-`null` ambiguity: `clear_*` wins, then an explicit value, else leave.
/// Re-targeting (e.g. dashboard → group) sends `target` + `clear_context: true`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateNavNodeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    /// Re-root the node (move to top level), overriding `parent_id`.
    #[serde(default)]
    pub clear_parent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<NavTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<NavContext>,
    /// Drop the context (e.g. when retargeting a dashboard mount to a group).
    #[serde(default)]
    pub clear_context: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub clear_icon: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default)]
    pub clear_accent: bool,
}
