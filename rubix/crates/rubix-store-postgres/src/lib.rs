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
//! Scope:
//!
//! - [`UNDO_SNAPSHOTS_MIGRATION_SOURCE`] — the
//!   `undo_snapshots` dimension table that backs every
//!   `Reversible` rubix tool. Snapshots are written before a
//!   destructive write; the retention sweep in
//!   `rubix-agent::boot::undo_sweep` prunes per
//!   `(tenant_id, resource_kind, resource_id)`.
//!
//! - [`FLOWS_DEFINITIONS_MIGRATION_SOURCE`] — the
//!   `flows_definitions` dimension table that holds every rubix
//!   flow definition. Bundled YAMLs are seeded on first boot per
//!   `(tenant_id, flow_id)`; subsequent edits land as new
//!   revisions and supersede prior heads via `superseded_at`.
//!   An insert/update trigger fires `NOTIFY rubix_flows_definitions`
//!   so the listener in `rubix-agent::boot::flow_notify` can
//!   reload the in-process `FlowRegistry` across instances
//!   without a redeploy.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod dashboards;
pub mod flows;

pub use dashboards::PgDashboardStore;
pub use flows::PgFlowDefStore;
pub use starter_store_postgres::MigrationSource;

/// `sqlx` migrator for the rubix `undo_snapshots` schema. Pair
/// with `starter_store_postgres::migrate(pool)
///     .with_source(UNDO_SNAPSHOTS_MIGRATION_SOURCE)` at boot.
pub static UNDO_SNAPSHOTS_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/undo");

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

/// `sqlx` migrator for the rubix `flows_definitions` schema. Pair
/// with `starter_store_postgres::migrate(pool)
///     .with_source(FLOWS_DEFINITIONS_MIGRATION_SOURCE)` at boot.
pub static FLOWS_DEFINITIONS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/flows_definitions");

/// Convenience [`MigrationSource`] for the `flows_definitions`
/// table. The `name` field becomes the suffix of the source's own
/// `_sqlx_migrations_flows_definitions` table, isolating this
/// schema's migration history from the other rubix- and starter-
/// owned sources in the boot chain.
pub const FLOWS_DEFINITIONS_MIGRATION_SOURCE: MigrationSource = MigrationSource {
    name: "flows_definitions",
    migrator: &FLOWS_DEFINITIONS_MIGRATOR,
};

/// Postgres channel name the `flows_definitions` insert/update
/// trigger publishes to. Kept here so the producer (the trigger
/// SQL) and the consumer (`rubix-agent::boot::flow_notify`) share
/// one source of truth.
pub const FLOWS_DEFINITIONS_CHANNEL: &str = "rubix_flows_definitions";

/// `sqlx` migrator for the rubix `dashboards_definitions` schema.
/// Pair with `starter_store_postgres::migrate(pool)
///     .with_source(DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE)` at boot.
pub static DASHBOARDS_DEFINITIONS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/dashboards_definitions");

/// Convenience [`MigrationSource`] for the `dashboards_definitions`
/// table (Goal 1, Phase A.1). The `name` field becomes the suffix
/// of the source's own `_sqlx_migrations_dashboards_definitions`
/// table, isolating this schema's migration history from the other
/// rubix- and starter-owned sources in the boot chain.
pub const DASHBOARDS_DEFINITIONS_MIGRATION_SOURCE: MigrationSource = MigrationSource {
    name: "dashboards_definitions",
    migrator: &DASHBOARDS_DEFINITIONS_MIGRATOR,
};

/// Postgres channel name the `dashboards_definitions` insert/update
/// trigger publishes to. Consumed by the Phase A.2 listener in
/// `rubix-agent::boot::dashboards_notify` to invalidate the
/// in-process `PageProvider` cache cross-instance.
pub const DASHBOARDS_DEFINITIONS_CHANNEL: &str = "rubix_dashboards_definitions";
