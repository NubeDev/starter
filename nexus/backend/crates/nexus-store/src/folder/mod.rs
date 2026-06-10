//! Tenant-scoped dashboard-folder persistence (WS-05).
//!
//! Folders organise dashboards into a nestable tree. Like dashboards, every
//! function runs inside a tenant-bound transaction so RLS isolates the rows.
//! Deleting a folder re-roots its children and filed dashboards rather than
//! cascading — losing the organisation must never destroy the contents.

mod delete;
mod fetch;
mod insert;
mod record;
mod update;

pub use delete::delete;
pub use fetch::{by_id, list};
pub use insert::{insert, insert_with_id};
pub use record::{FolderPatch, FolderRecord, NewFolder};
pub use update::update;
