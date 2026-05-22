//! SQLite implementations of the `starter-flow-spi` persistence
//! seams (D-F3.3).
//!
//! Gated behind the default-off `flow` cargo feature so the
//! `starter-store-sqlite` baseline is unchanged for every existing
//! consumer (D-F3.7). The flow migrations live under
//! `migrations/flow/` and are exposed as [`FLOW_MIGRATOR`] for the
//! namespaced migration runner.
//!
//! Three impls, one per SPI trait:
//!
//! - [`flow_store::SqliteFlowStore`] — flow definitions + revisions
//!   + per-flow head pointer.
//! - [`run_store::SqliteRunStore`] — runs, per-tick checkpoints,
//!   dedup lookup. Checkpoint insert + retention prune happen in
//!   one `BEGIN IMMEDIATE` transaction (D-F3.8 + D-F3.9).
//! - [`session_store::SqliteSessionStore`] — session records,
//!   principal-scoped listing.

pub mod agent_session_store;
pub mod flow_store;
pub mod run_store;
mod schema;
pub mod session_store;

pub use agent_session_store::SqliteAgentSessionStore;
pub use flow_store::SqliteFlowStore;
pub use run_store::SqliteRunStore;
pub use session_store::SqliteSessionStore;

/// `sqlx` migrator for the flow persistence schema. Pair with the
/// crate's `migrate(pool).with_source(MigrationSource { name:
/// "flow", migrator: FLOW_MIGRATOR })` chain.
pub static FLOW_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/flow");

/// Convenience `MigrationSource` for the flow schema. Add it to
/// the `migrate(pool)` chain on engine boot.
pub const FLOW_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "flow",
        migrator: &FLOW_MIGRATOR,
    };

/// Migrator for the agent-session persistence schema
/// (DOCS/agent/MEMORY.md Phase M-A). Ships its own three-table
/// schema (`agent_sessions`, `agent_session_turns`,
/// `agent_session_artifacts`) on a dedicated migration table so
/// consumers can adopt it without pulling the full flow schema
/// (which owns a `runs` table that conflicts with bespoke
/// per-app schemas — flow-agent ships its own).
pub static AGENT_SESSION_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/agent_sessions");

/// Convenience `MigrationSource` for the agent-session schema only.
pub const AGENT_SESSION_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "agent_sessions",
        migrator: &AGENT_SESSION_MIGRATOR,
    };
