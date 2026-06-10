//! Saved-flow execution: the manager that runs light ingestion pipelines, the
//! per-flow ingest metrics, the failure policy, and the metered node wrappers
//! that enforce both.

pub mod manager;
pub mod metered;
pub mod metrics;
pub mod policy;

pub use manager::{FlowManager, FlowStats};
pub use metrics::{FlowMetrics, MetricsSnapshot};
