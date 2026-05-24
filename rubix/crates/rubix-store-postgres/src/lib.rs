//! # rubix-store-postgres
//!
//! Rubix-owned Postgres schemas that sit on top of the generic
//! `starter-store-postgres` building blocks. The crate exists so
//! the migrations live next to the rubix domain code that reads
//! and writes the tables, while still routing through the
//! namespaced [`starter_store_postgres::migrate`] runner so each
//! source records progress in its own `_sqlx_migrations_<name>`
//! table (per starter R4 — no version-number collisions across
//! migration sources).
//!
//! Scope (per `rubix/SCOPE.md` Phase A):
//!
//! - [`UNDO_SNAPSHOTS_MIGRATION_SOURCE`] — the
//!   `undo_snapshots` dimension table that backs every
//!   `Reversible` rubix tool. Snapshots are written before a
//!   destructive write; the retention sweep in
//!   `rubix-agent::boot::undo_sweep` prunes per
//!   `(tenant_id, resource_kind, resource_id)`.
//!
//! Future rubix-owned schemas (e.g. `flows_definitions` for
//! Phase D) land here as additional `MigrationSource` constants.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use starter_store_postgres::MigrationSource;

/// `sqlx` migrator for the rubix `undo_snapshots` schema. Pair
/// with `starter_store_postgres::migrate(pool)
///     .with_source(UNDO_SNAPSHOTS_MIGRATION_SOURCE)` at boot.
pub static UNDO_SNAPSHOTS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/undo");

/// Convenience [`MigrationSource`] for the `undo_snapshots`
/// table. The `name` field becomes the suffix of the source's
/// own `_sqlx_migrations_undo_snapshots` table, isolating the
/// rubix-owned migration history from the starter-owned sources
/// already in the chain (`starter`, `auth_users`, `changelog`,
/// …).
pub const UNDO_SNAPSHOTS_MIGRATION_SOURCE: MigrationSource = MigrationSource {
    name: "undo_snapshots",
    migrator: &UNDO_SNAPSHOTS_MIGRATOR,
};
