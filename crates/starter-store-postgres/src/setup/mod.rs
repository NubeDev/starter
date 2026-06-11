//! Postgres implementations of the `starter-setup-spi` store seams
//! (DOCS §5). Postgres twin of the SQLite `setup` module: JSONB columns,
//! `TIMESTAMPTZ`, `BOOLEAN`, `$N` placeholders.
//!
//! Gated behind the default-off `setup` cargo feature.

pub mod run_store;
pub mod template_store;

pub use run_store::PgSetupRunStore;
pub use template_store::PgTemplateStore;

/// `sqlx` migrator for the setup catalog schema. Pair with
/// `migrate(pool).with_source(SETUP_MIGRATION_SOURCE)`.
pub static SETUP_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/setup");

/// Convenience `MigrationSource` for the setup schema (DOCS §5).
pub const SETUP_MIGRATION_SOURCE: crate::migrate::MigrationSource =
    crate::migrate::MigrationSource {
        name: "setup",
        migrator: &SETUP_MIGRATOR,
    };

/// Combined bindings payload persisted in the single `bindings` JSONB
/// column.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct StoredBindings {
    pub input: Vec<starter_setup_spi::model::InputBinding>,
    pub output: Vec<starter_setup_spi::model::OutputBinding>,
}

/// Parse an RFC3339 string (the SPI timestamp form) into a UTC datetime
/// for `TIMESTAMPTZ` binding; falls back to "now" semantics by erroring
/// to the caller on malformed input.
pub(crate) fn parse_ts(s: &str) -> Result<chrono::DateTime<chrono::Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|e| format!("timestamp {s}: {e}"))
}

/// Render a `TIMESTAMPTZ` back to the SPI's RFC3339 string form.
pub(crate) fn fmt_ts(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.to_rfc3339()
}
