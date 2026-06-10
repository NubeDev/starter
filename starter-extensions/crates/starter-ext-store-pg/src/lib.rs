//! PostgreSQL-backed [`EnablementStore`] implementation.
//!
//! See `DOCS/extensions/scope/SCOPE.md` "Decisions made — enable/disable
//! persistence model": one DB row per extension id, queried at host boot.
//! This crate is the first concrete impl of the
//! [`starter_ext_server::EnablementStore`] trait beyond the in-memory
//! default; rubix-agent consumes it via the wiring landed in the
//! `rubix-extensions-wire` job.
//!
//! The crate is intentionally tiny: one type (`PgEnablementStore`), one
//! migration (`migrations/0001_extensions_enablement.sql`), and the
//! trait impl. Schema is owned here — callers run the migration via
//! sqlx's standard `Migrator` against this crate's `migrations/` dir.
//!
//! [`EnablementStore`]: starter_ext_server::EnablementStore

#![deny(missing_docs)]

mod store;

pub use store::PgEnablementStore;

/// The sqlx [`Migrator`] over this crate's `migrations/` dir.
///
/// Exposed so a consumer can chain the `extensions_enablement` schema into
/// its own migration plan without re-declaring the SQL. The host owns the
/// namespacing: wrap this in whatever migration-runner type the host uses
/// (nexus wraps it as a `starter_store_postgres::MigrationSource` named
/// `ext_store`). Keeping it a bare `Migrator` keeps this crate decoupled
/// from any one host's migration-runner type.
///
/// [`Migrator`]: sqlx::migrate::Migrator
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./src/migrations");
