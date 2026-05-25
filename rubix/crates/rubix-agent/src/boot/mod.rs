//! Process startup helpers — one verb per file.

pub mod ai;
pub mod auth;
pub mod authz;
pub mod clickhouse;
pub mod config;
pub mod dashboards_seed;
pub mod extensions;
pub mod extensions_flow;
pub mod flow_notify;
pub mod flow_runtime;
pub mod flows_seed;
pub mod mcp;
pub mod migrations;
pub mod scheduler;
pub mod sdui;
pub mod tracing;
pub mod undo_sweep;

pub use auth::{build_auth, AuthSurface};
pub use clickhouse::{apply_ch_migrations, rubix_ch_config, RUBIX_CH_DATABASE};
pub use config::{
    AgentConfig, ExtensionsConfig, FlowRuntimeConfig, SchedulerConfig, UndoConfig,
};
pub use flow_runtime::{
    build as build_flow_runtime, bundled_schedule_pairs, BundledSchedule, FlowRuntime,
    FlowSubscriptionRegistry,
};
pub use extensions::{
    build_extension_admin, BootError as ExtensionsBootError, ExtensionAdminBundle,
    SYSTEM_AUTOSTART_PRINCIPAL,
};
pub use extensions_flow::{register_contributed_nodes, ExtensionsFlowError};
pub use scheduler::{spawn as spawn_scheduler, SchedulerHandle};
pub use sdui::build_sdui_router;
pub use flow_notify::spawn_flow_notify;
pub use flows_seed::seed_and_load as seed_and_load_flow_definitions;
pub use migrations::apply_migrations;
pub use tracing::init_tracing;
pub use undo_sweep::{spawn_undo_sweep, sweep_once as sweep_undo_once};
