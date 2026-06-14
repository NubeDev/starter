//! Dashboard-folder DTOs (WS-05). Folders organise dashboards into a nestable
//! tree; they key on an immutable id and carry a nullable `parent_id` (NULL is
//! the root).

mod create;
mod get;
mod update;

pub use create::CreateFolderRequest;
pub use get::FolderSummary;
pub use update::UpdateFolderRequest;
