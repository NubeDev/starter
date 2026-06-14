//! Response body of `POST /api/v1/undo` and `POST /api/v1/redo`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The changelog group that an undo/redo applied. The client refreshes the
/// resources touched by this group (it can fetch the group's rows via the audit
/// API if it needs the per-resource detail).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UndoResponse {
    /// The `group_id` that was undone or redone.
    pub group_id: String,
}
