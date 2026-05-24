//! Process startup helpers — one verb per file.

pub mod auth;
pub mod authz;
pub mod clickhouse;
pub mod config;
pub mod mcp;
pub mod migrations;
pub mod tracing;

pub use auth::{build_auth, AuthSurface};
pub use clickhouse::apply_ch_migrations;
pub use config::AgentConfig;
pub use migrations::apply_migrations;
pub use tracing::init_tracing;
