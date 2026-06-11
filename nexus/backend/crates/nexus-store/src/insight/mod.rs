//! Tenant-scoped stored-insight persistence (RW-06).
//!
//! An insight is a named, reusable post-query transform script. Like dashboards
//! and folders, every function runs inside a tenant-bound transaction so RLS
//! isolates the rows; a panel references an insight by id and the query path
//! resolves it under the same tenant scope before running it sandboxed.

mod delete;
mod fetch;
mod insert;
mod record;
mod update;

pub use delete::delete;
pub use fetch::{by_id, list};
pub use insert::insert;
pub use record::{InsightPatch, InsightRecord, NewInsight};
pub use update::update;
