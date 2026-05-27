//! Process startup helpers — one verb per file.

pub mod ai;
pub mod auth;
pub mod authz;
pub mod config;
pub mod dashboards_seed;
pub mod extensions;
pub mod extensions_flow;
pub mod flow_notify;
pub mod flow_runtime;
pub mod flows_seed;
pub mod mcp;
pub mod migrations;
pub mod pool_telemetry;
pub mod runtime_canary;
pub mod runtime_metrics;
pub mod scheduler;
pub mod task_watchdog;
pub mod sdui;
pub mod tracing;
pub mod undo_sweep;
pub mod warehouse;

pub use auth::{build_auth, AuthSurface};
pub use config::{AgentConfig, ExtensionsConfig, FlowRuntimeConfig, SchedulerConfig, UndoConfig};
pub use extensions::{
    build_extension_admin, BootError as ExtensionsBootError, ExtensionAdminBundle,
    SYSTEM_AUTOSTART_PRINCIPAL,
};
pub use extensions_flow::{register_contributed_nodes, ExtensionsFlowError};
pub use flow_notify::spawn_flow_notify;
pub use flow_runtime::{
    build as build_flow_runtime, bundled_schedule_pairs, BundledSchedule, FlowRuntime,
    FlowSubscriptionRegistry,
};
pub use flows_seed::seed_and_load as seed_and_load_flow_definitions;
pub use migrations::apply_migrations;
pub use scheduler::{spawn as spawn_scheduler, SchedulerHandle};
pub use sdui::build_sdui_router;
pub use tracing::init_tracing;
pub use undo_sweep::{spawn_undo_sweep, sweep_once as sweep_undo_once};
pub use warehouse::{apply_warehouse_migrations, connect_warehouse, RUBIX_CH_DATABASE};
