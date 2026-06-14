//! `GET /api/v1/agents` response item.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// An agent as it appears in the list: identity and backend/model, without the
/// config blob (that comes from the detail endpoint).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AgentSummary {
    pub id: Uuid,
    pub name: String,
    pub backend: String,
    pub model: String,
}
