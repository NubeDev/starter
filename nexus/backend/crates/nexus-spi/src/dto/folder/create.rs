//! `POST /api/v1/folders` — create a folder.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Create a folder. `parent_id` files it under another folder (in the same
/// tenant); omit it for a root folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateFolderRequest {
    pub name: String,
    /// Parent folder id; omit/`null` for a root folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
}
