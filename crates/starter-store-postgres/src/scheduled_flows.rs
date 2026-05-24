//! Migration source for the `starter_scheduled_flows` dimension
//! table (Phase A.2 of the durable scheduler land).
//!
//! Schema-only crate-side surface for now — the typed CRUD layer
//! and the `FlowAsService` tick loop land in
//! `starter-flow-surfaces` in Phase B and consume the same
//! migrator via [`SCHEDULED_FLOWS_MIGRATION_SOURCE`].

/// `sqlx` migrator for the scheduled-flows schema. Pair with the
/// crate's
/// `migrate(pool).with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)`
/// chain on engine boot.
pub static SCHEDULED_FLOWS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/scheduled_flows");

/// Convenience [`MigrationSource`] for the scheduled-flows schema.
///
/// [`MigrationSource`]: crate::migrate::MigrationSource
pub const SCHEDULED_FLOWS_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "scheduled_flows",
        migrator: &SCHEDULED_FLOWS_MIGRATOR,
    };
