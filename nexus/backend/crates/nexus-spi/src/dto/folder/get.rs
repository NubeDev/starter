//! `GET /api/v1/folders` — a folder in the tenant's tree.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// One folder: its immutable id, its parent (NULL = root), and display name. The
/// client assembles the tree from the flat list using `parent_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FolderSummary {
    pub id: Uuid,
    /// Parent folder id; `null` for a root folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    pub name: String,
}
