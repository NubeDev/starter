//! Process startup helpers — one verb per file.

pub mod ai;
pub mod auth;
pub mod authz;
pub mod clickhouse;
pub mod config;
pub mod flow_notify;
pub mod flows_seed;
pub mod mcp;
pub mod migrations;
pub mod tracing;
pub mod undo_sweep;

pub use auth::{build_auth, AuthSurface};
pub use clickhouse::{apply_ch_migrations, rubix_ch_config, RUBIX_CH_DATABASE};
pub use config::{AgentConfig, UndoConfig};
pub use flow_notify::spawn_flow_notify;
pub use flows_seed::seed_and_load as seed_and_load_flow_definitions;
pub use migrations::apply_migrations;
pub use tracing::init_tracing;
pub use undo_sweep::{spawn_undo_sweep, sweep_once as sweep_undo_once};
