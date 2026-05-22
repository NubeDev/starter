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
//! Three impls, one per SPI trait:
//!
//! - [`flow_store::PgFlowStore`] — flow definitions + revisions
//!   + per-flow head pointer.
//! - [`run_store::PgRunStore`] — runs, per-tick checkpoints,
//!   dedup lookup. Checkpoint insert + retention prune happen in
//!   one transaction (D-F3.8 + D-F3.9).
//! - [`session_store::PgSessionStore`] — session records,
//!   principal-scoped listing.

pub mod flow_store;
pub mod run_store;
mod schema;
pub mod session_store;

pub use flow_store::PgFlowStore;
pub use run_store::PgRunStore;
pub use session_store::PgSessionStore;

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
