//! `GET /api/v1/flows` response item.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A flow as it appears in the list: identity and run state, without the config
/// blobs (those come from the detail endpoint).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FlowSummary {
    pub id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub running: bool,
}
