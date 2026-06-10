//! Stored-insight DTOs (RW-06). A tenant-scoped insight is a named, reusable
//! post-query transform script; panels reference one by id. The CRUD shape
//! mirrors the folder vertical — flat list, immutable id, RLS isolation.

mod create;
mod get;
mod query_ref;
mod update;

pub use create::CreateInsightRequest;
pub use get::InsightSummary;
pub use query_ref::InsightRef;
pub use update::UpdateInsightRequest;
