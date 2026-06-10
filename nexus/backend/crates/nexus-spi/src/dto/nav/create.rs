//! Create-nav-node request (WS-13 §4).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::shared::{NavContext, NavTarget};

/// Create a nav node. `target` defaults to a `group` header when omitted so a
/// client can lay out structure first and bind pages later. `context` is only
/// meaningful for a `dashboard` target and is ignored (cleared) otherwise.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateNavNodeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    pub title: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default = "default_target")]
    pub target: NavTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<NavContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
}

fn default_target() -> NavTarget {
    NavTarget::Group
}
