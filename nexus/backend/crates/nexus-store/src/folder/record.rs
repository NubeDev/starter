//! Folder records and their create/update inputs.

use uuid::Uuid;

/// A stored dashboard folder. Nestable via `parent_id` (NULL = root).
#[derive(Debug, Clone)]
pub struct FolderRecord {
    pub id: Uuid,
    pub tenant_id: String,
    pub parent_id: Option<Uuid>,
    pub name: String,
}

/// Input to create a folder.
#[derive(Debug, Clone)]
pub struct NewFolder {
    pub parent_id: Option<Uuid>,
    pub name: String,
}

/// Partial update of a folder. `None` leaves a field untouched (COALESCE in the
/// store). `parent_id` reparents the folder; a `Some(None)` (re-rooting) is not
/// expressible through COALESCE, so reparent-to-root is handled explicitly.
#[derive(Debug, Clone, Default)]
pub struct FolderPatch {
    pub name: Option<String>,
    /// `Some(parent)` moves under `parent`; `Some(None)` re-roots; `None` leaves
    /// the parent unchanged. Wrapped so the three cases are distinguishable.
    pub parent_id: Option<Option<Uuid>>,
}
