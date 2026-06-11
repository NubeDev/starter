//! Stored-insight DTOs (RW-06). A tenant-scoped insight is a named, reusable
//! post-query transform script; panels reference one by id. The CRUD shape
//! mirrors the folder vertical — flat list, immutable id, RLS isolation.

mod create;
mod functions;
mod get;
mod preview;
mod query_ref;
mod update;

pub use create::CreateInsightRequest;
pub use functions::{InsightFunctionCatalog, InsightFunctionDoc};
pub use get::InsightSummary;
pub use preview::{PreviewInsightError, PreviewInsightRequest, PreviewInsightResponse};
pub use query_ref::InsightRef;
pub use update::UpdateInsightRequest;
