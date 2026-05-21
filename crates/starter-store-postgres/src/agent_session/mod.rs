//! Postgres implementation of the [`AgentSessionStore`] trait
//! (DOCS/agent/MEMORY.md Phase M-B). Twin of the SQLite impl —
//! same shape, same caps, same M5 concurrency contract; only the
//! storage layer differs.
//!
//! Gated behind the default-off `agent-session` cargo feature so the
//! `starter-store-postgres` baseline is unchanged for every existing
//! consumer (mirrors the SQLite `flow` feature split). The
//! migrations live under `migrations/agent_sessions/` and ship a
//! dedicated migration table so apps that own their own `runs` /
//! `sessions` schemas can adopt agent persistence without collision.
//!
//! [`AgentSessionStore`]: starter_flow_spi::agent_session::AgentSessionStore

mod agent_session_store;

pub use agent_session_store::PostgresAgentSessionStore;

/// `sqlx` migrator for the agent-session schema. Pair with the
/// crate's `migrate(pool).with_source(AGENT_SESSION_MIGRATION_SOURCE)`
/// chain.
pub static AGENT_SESSION_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/agent_sessions");

/// Convenience `MigrationSource` for the agent-session schema only.
pub const AGENT_SESSION_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "agent_sessions",
        migrator: &AGENT_SESSION_MIGRATOR,
    };
