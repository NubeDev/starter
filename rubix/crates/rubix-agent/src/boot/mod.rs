//! Process startup helpers — one verb per file.

pub mod clickhouse;
pub mod mcp;
pub mod migrations;
pub mod tracing;

pub use clickhouse::apply_ch_migrations;
pub use migrations::apply_migrations;
pub use tracing::init_tracing;
