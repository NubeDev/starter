//! SQLite implementations of the `starter-setup-spi` store seams
//! (DOCS §5).
//!
//! Gated behind the default-off `setup` cargo feature so the
//! `starter-store-sqlite` baseline is unchanged for consumers that do
//! not run the automation builder. The setup migrations live under
//! `migrations/setup/` and are exposed as [`SETUP_MIGRATOR`] for the
//! namespaced migration runner.
//!
//! Two impls:
//!
//! - [`template_store::SqliteTemplateStore`] — the template catalog,
//!   keyed `(tenant_id, id, version)` with the `__global__` sentinel.
//! - [`run_store::SqliteSetupRunStore`] — the thin run index over flow
//!   `RunId`s.

pub mod run_store;
pub mod template_store;

pub use run_store::SqliteSetupRunStore;
pub use template_store::SqliteTemplateStore;

/// `sqlx` migrator for the setup catalog schema. Pair with
/// `migrate(pool).with_source(SETUP_MIGRATION_SOURCE)`.
pub static SETUP_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/setup");

/// Convenience `MigrationSource` for the setup schema. Compose alongside
/// `FLOW_MIGRATION_SOURCE` on engine boot (DOCS §5).
pub const SETUP_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "setup",
        migrator: &SETUP_MIGRATOR,
    };

/// Combined bindings payload persisted in the single `bindings` JSON
/// column (the SPI splits input/output for ergonomics; the catalog
/// stores them together).
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct StoredBindings {
    pub input: Vec<starter_setup_spi::model::InputBinding>,
    pub output: Vec<starter_setup_spi::model::OutputBinding>,
}
