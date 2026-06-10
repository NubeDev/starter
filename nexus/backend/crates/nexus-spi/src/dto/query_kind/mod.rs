//! Query-kind DTOs — tenant-authored named SQL queries (Create/Update/Detail).
//!
//! The catalogue's read-only `QueryKindSummary`/`QueryKindList` live in
//! [`crate::dto::query::kinds`]; these are the authoring verbs for a tenant's own
//! kinds and carry the SQL the catalogue hides.

pub mod create;
pub mod shared;
pub mod update;

pub use create::CreateQueryKindRequest;
pub use shared::QueryKindDetail;
pub use update::UpdateQueryKindRequest;
