//! Flow DTOs — saved ingestion pipelines.

pub mod create;
pub mod list;
pub mod shared;
pub mod update;

pub use create::CreateFlowRequest;
pub use list::FlowSummary;
pub use shared::FlowDetail;
pub use update::UpdateFlowRequest;
