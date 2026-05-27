//! Phase B.1 host glue for the upstream SDUI router.
//!
//! Per `rubix/docs/scope/dashboards/03-host-glue.md`, rubix ships
//! four trait impls — one per file — that let
//! `starter-sdui-routes::sdui_router` answer requests against the
//! rubix data plane (Postgres for pages + dimensional data,
//! ClickHouse for history, the tool registry for `/action`).
//!
//! Subsequent stages will mount these via `boot/sdui.rs` and add the
//! `MessageCatalogue` (G6), the NOTIFY-driven page cache, and the
//! `WritePlanAcl` policy seam. This module only ships the four trait
//! impls and their unit-test seams.

pub mod analytics_bridge;
pub mod entity_graph;
pub mod handler_registry;
pub mod page_provider;
pub mod query_engine;
pub mod template_resolver;

pub use analytics_bridge::TimescaleAnalyticsBridge;
pub use entity_graph::RubixEntityGraph;
pub use handler_registry::{tool_action_handler, RubixHandlerRegistry};
pub use page_provider::PgPageProvider;
pub use query_engine::RubixQueryEngine;
