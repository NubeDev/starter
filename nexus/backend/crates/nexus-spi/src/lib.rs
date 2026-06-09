//! Nexus control-plane contracts.
//!
//! Wire types (REST DTOs decorated for OpenAPI) and id re-exports. This crate is
//! the seam the frontend codegens its client from, so it depends only on
//! `starter-spi` and serialization crates — never on the engine, the store, or
//! the binary. Surfaces are add-only within a major.
//!
//! Errors travel as `starter_spi`'s [`Problem`] document, mapped from
//! `starter_spi::Error` by `starter_server` at the transport edge; nexus does
//! not define a second error shape.

pub mod dto;
pub mod id;
pub mod openapi;

pub use id::{AlertRuleId, DashboardId, DatasourceId, FlowId, PanelId, StreamId};

/// The error body every nexus endpoint returns on failure — reused from
/// `starter-spi` so the whole platform speaks one error shape.
pub use starter_spi::dto::Problem;
/// Cursor-paged collection envelope, reused from `starter-spi`.
pub use starter_spi::paging::{Cursor, Page};
