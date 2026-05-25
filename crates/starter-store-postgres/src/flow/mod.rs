//! Postgres implementations of the `starter-flow-spi` persistence
//! seams. Twin of `starter-store-sqlite::flow` — same shape, same
//! trait coverage, same D-F3.3/8/9/12 invariants; only the SQL
//! dialect and column types differ.
//!
//! Gated behind the default-off `flow` cargo feature so the
//! `starter-store-postgres` baseline is unchanged for every existing
//! consumer (mirrors the SQLite `flow` feature split). The flow
//! migrations live under `migrations/flow/` and are exposed as
//! [`FLOW_MIGRATION_SOURCE`] for the namespaced migration runner.
//!
//! Three core impls, one per SPI trait:
//!
//! - [`flow_store::PgFlowStore`] — flow definitions + revisions
//!   + per-flow head pointer.
//! - [`run_store::PgRunStore`] — runs, per-tick checkpoints,
//!   dedup lookup. Checkpoint insert + retention prune happen in
//!   one transaction (D-F3.8 + D-F3.9).
//! - [`session_store::PgSessionStore`] — session records,
//!   principal-scoped listing.
//!
//! Plus, gated behind the additive `agent-session` feature
//! (DOCS/storage/ADR-001), the M-B persistence seam:
//!
//! - [`agent_session_store::PgAgentSessionStore`] — append-only
//!   turns + versioned artifacts, ships its own three-table schema
//!   under `migrations/agent_sessions/`.
//!
//! `agent-session` implies `flow` so the layout mirrors the SQLite
//! twin one-for-one (where both stores live under `flow/` and are
//! gated by a single `flow` feature). See `Cargo.toml`.

pub mod flow_store;
pub mod node_state;
pub mod run_store;
mod schema;
pub mod session_store;

#[cfg(feature = "agent-session")]
pub mod agent_session_store;

pub use flow_store::PgFlowStore;
pub use node_state::PgNodeStateStore;
pub use run_store::PgRunStore;
pub use session_store::PgSessionStore;

#[cfg(feature = "agent-session")]
pub use agent_session_store::PgAgentSessionStore;

/// `sqlx` migrator for the flow persistence schema. Pair with the
/// crate's `migrate(pool).with_source(FLOW_MIGRATION_SOURCE)`
/// chain.
pub static FLOW_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/flow");

/// Convenience `MigrationSource` for the flow schema. Add it to
/// the `migrate(pool)` chain on engine boot.
pub const FLOW_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "flow",
        migrator: &FLOW_MIGRATOR,
    };

/// Migrator for the agent-session persistence schema
/// (DOCS/agent/MEMORY.md Phase M-B / DOCS/storage/ADR-001). Ships
/// its own three-table schema (`agent_sessions`,
/// `agent_session_turns`, `agent_session_artifacts`) on a
/// dedicated migration table so consumers can adopt it without
/// pulling the full flow schema (which owns a `runs` table that
/// conflicts with bespoke per-app schemas — flow-agent ships its
/// own).
#[cfg(feature = "agent-session")]
pub static AGENT_SESSION_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/agent_sessions");

/// Convenience `MigrationSource` for the agent-session schema only.
#[cfg(feature = "agent-session")]
pub const AGENT_SESSION_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "agent_sessions",
        migrator: &AGENT_SESSION_MIGRATOR,
    };
