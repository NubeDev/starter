//! A named set of migrations. Each consumer of the database registers
//! one — `starter`, `starter_auth`, `app`, etc. — and the runner
//! keeps version state per-source.

/// One migration source: a name + a `sqlx::migrate::Migrator` borrow.
///
/// Build with `sqlx::migrate!("relative/path/to/migrations")` from
/// the consumer crate. The macro produces a `static Migrator`, which
/// the source borrows; this avoids requiring `Migrator: Clone` (sqlx
/// 0.8 doesn't implement it).
///
/// Pass the same name consistently across runs — the name is the
/// lookup key for the source-scoped migrations table.
#[derive(Copy, Clone)]
pub struct MigrationSource {
    /// Source identifier. Must match `^[a-z][a-z0-9_]{0,30}$` —
    /// goes straight into a SQL identifier.
    pub name: &'static str,

    /// The sqlx migrator over the consumer's migration files.
    /// Typically `&MY_MIGRATOR` where `static MY_MIGRATOR =
    /// sqlx::migrate!(...)`.
    pub migrator: &'static sqlx::migrate::Migrator,
}
