//! `PATCH /api/v1/folders/:id` — rename or reparent a folder.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Partial update. `name` renames. The parent is three-valued, modelled
/// explicitly so a JSON-friendly wire shape (no double-`null` ambiguity) can
/// distinguish the cases: send `parent_id` to move under a folder, set
/// `clear_parent: true` to re-root, or send neither to leave the parent
/// unchanged. `clear_parent` wins if both are set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateFolderRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Move under this folder. Ignored when `clear_parent` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    /// Re-root the folder (set its parent to NULL).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clear_parent: bool,
}
