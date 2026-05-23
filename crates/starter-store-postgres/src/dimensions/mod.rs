//! Warehouse dimensions feature (W1, W5, W12, W15).
//!
//! Gated by the `dimensions` cargo feature. Adds the eight catalog
//! tables (`entities`, `entity_refs`, `tag_definitions`,
//! `tag_prefix_registry`, `marts`, `cleaners`, `sandboxes`,
//! `ext_manifest_approvals`) under the dedicated migration
//! namespace `_sqlx_migrations_dimensions`.
//!
//! Submodules expose typed CRUD plus the partial-index/CHECK
//! enforcement helpers. Public surface is intentionally small —
//! `starter-warehouse` is the orchestrator; this crate just owns
//! the Postgres seam.

// Submodules ship row structs whose fields mirror the catalog
// column names one-for-one. The columns are documented in the
// migration SQL; re-stating the same docs on every Rust field is
// noise, not signal. Doc-lint is silenced inside the module.
#![allow(missing_docs)]

pub mod catalog_audit;
pub mod catalog_gc;
pub mod cleaners;
pub mod entities;
pub mod entity_refs;
pub mod ext_manifest_approvals;
pub mod marts;
pub mod sandboxes;
pub mod tag_definitions;
pub mod tag_prefix_registry;

/// `sqlx` migrator for the dimensions schema. Paired with the
/// dedicated `_sqlx_migrations_dimensions` version table.
pub static DIMENSIONS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/dimensions");

/// Convenience `MigrationSource` for the dimensions schema. Mount
/// on the migrate chain via
/// `migrate(pool).with_source(DIMENSIONS_MIGRATION_SOURCE)`.
pub const DIMENSIONS_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "dimensions",
        migrator: &DIMENSIONS_MIGRATOR,
    };
