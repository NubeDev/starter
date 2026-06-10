//! Flow DTOs — saved ingestion pipelines.

pub mod create;
pub mod dry_run;
pub mod export;
pub mod list;
pub mod node_type;
pub mod shared;
pub mod update;

pub use create::CreateFlowRequest;
pub use dry_run::{DryRunRequest, DryRunResponse};
pub use export::{redact_secrets, FlowExport, FLOW_SCHEMA_VERSION};
pub use list::FlowSummary;
pub use node_type::{NodeCategory, NodeType, NodeTypeList};
pub use shared::{FlowDetail, FlowMetrics};
pub use update::UpdateFlowRequest;
