//! Notification-channel request/response DTOs.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// Create a notification channel. `kind` selects the delivery (webhook in v1);
/// `config` is the kind-specific settings (e.g. `{ "url": "…" }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateChannelRequest {
    pub name: String,
    pub kind: String,
    pub config: Value,
}

/// A notification channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ChannelDetail {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub config: Value,
}
