//! A named set of migrations. Each consumer of the database registers
//! one — `starter`, `starter_auth`, `app`, etc. — and the runner
//! keeps version state per-source.

/// One migration source: a name + a `sqlx::migrate::Migrator`.
///
/// Build with `sqlx::migrate!("relative/path/to/migrations")` from
/// the consumer crate. Pass the same name consistently across runs
/// — the name is the lookup key for the source-scoped migrations
/// table.
pub struct MigrationSource {
    /// Source identifier. Must match `^[a-z][a-z0-9_]{0,30}$` —
    /// goes straight into a SQL identifier.
    pub name: &'static str,

    /// The sqlx migrator over the consumer's migration files.
    pub migrator: sqlx::migrate::Migrator,
}
