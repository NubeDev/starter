//! REST data-transfer objects, one folder per resource noun, one file per verb.
//!
//! Every type here derives `serde` + `utoipa::ToSchema` and is part of the
//! published OpenAPI contract. Ids cross the wire as `uuid::Uuid` because the
//! internal phantom-typed `Id<T>` does not implement `ToSchema`.
//!
//! Dashboards, panels, alerts, and flows arrive with their milestones (M2/M3)
//! and extend this module add-only.

pub mod agent;
pub mod ai;
pub mod audit;
pub mod dashboard;
pub mod datasource;
pub mod detection;
pub mod flow;
pub mod folder;
pub mod ingest;
pub mod insight;
pub mod me;
pub mod nav;
pub mod panel;
pub mod query;
pub mod query_history;
pub mod query_kind;
pub mod stream;
pub mod tag;
pub mod variable;
